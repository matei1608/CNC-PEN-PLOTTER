#![no_std]
#![no_main]

use core::str;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// Legăm întreruperile necesare pentru funcționarea asincronă a portului serial USART1
bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

// ==========================================
// 1. DEFINIREA COMENZILOR G-CODE
// ==========================================
#[derive(Clone, Copy)]
enum Command {
    PenUp,
    PenDown,
    MoveTo(f32, f32), // Coordonate X și Y în milimetri
    Unknown,          // Pentru comenzi pe care le ignorăm
}

// ==========================================
// 2. CALIBRAREA
// ==========================================
const STEPS_PER_REV: f32 = 4096.0; 
const PINION_CIRCUMFERENCE_MM: f32 = 48.0; 
const STEPS_PER_MM: f32 = STEPS_PER_REV / PINION_CIRCUMFERENCE_MM;
const Z_STEPS_MOVE: usize = 300; 

const STEP_SEQUENCE: [[bool; 4]; 8] = [
    [true,  false, false, false],
    [true,  true,  false, false],
    [false, true,  false, false],
    [false, true,  true,  false],
    [false, false, true,  false],
    [false, false, true,  true ],
    [false, false, false, true ],
    [true,  false, false, true ],
];

// ==========================================
// STRUCTURĂ PENTRU CONTROLUL MOTOARELOR
// ==========================================
struct StepperMotor {
    in1: Output<'static>,
    in2: Output<'static>,
    in3: Output<'static>,
    in4: Output<'static>,
    step_index: usize,
}

impl StepperMotor {
    fn new(in1: Output<'static>, in2: Output<'static>, in3: Output<'static>, in4: Output<'static>) -> Self {
        Self { in1, in2, in3, in4, step_index: 0 }
    }

    fn step_once(&mut self, forward: bool) {
        if forward {
            self.step_index = (self.step_index + 1) % 8;
        } else {
            self.step_index = (self.step_index + 7) % 8; 
        }

        let step_state = STEP_SEQUENCE[self.step_index];
        if step_state[0] { self.in1.set_high(); } else { self.in1.set_low(); }
        if step_state[1] { self.in2.set_high(); } else { self.in2.set_low(); }
        if step_state[2] { self.in3.set_high(); } else { self.in3.set_low(); }
        if step_state[3] { self.in4.set_high(); } else { self.in4.set_low(); }
    }
    
    fn stop(&mut self) {
        self.in1.set_low();
        self.in2.set_low();
        self.in3.set_low();
        self.in4.set_low();
    }
}

// ==========================================
// 3. CONTROLUL AXELOR Z, X, Y
// ==========================================
async fn pen_up(motor_z: &mut StepperMotor) {
    info!("Executam: PEN UP");
    for _ in 0..Z_STEPS_MOVE {
        motor_z.step_once(true); 
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_z.stop();
}

async fn pen_down(motor_z: &mut StepperMotor) {
    info!("Executam: PEN DOWN");
    for _ in 0..Z_STEPS_MOVE {
        motor_z.step_once(false);
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_z.stop();
}

async fn draw_line(
    motor_x: &mut StepperMotor,
    motor_y: &mut StepperMotor,
    current_x_mm: &mut f32,
    current_y_mm: &mut f32,
    target_x_mm: f32,
    target_y_mm: f32,
) {
    let start_x_steps = (*current_x_mm * STEPS_PER_MM) as i32;
    let start_y_steps = (*current_y_mm * STEPS_PER_MM) as i32;
    let target_x_steps = (target_x_mm * STEPS_PER_MM) as i32;
    let target_y_steps = (target_y_mm * STEPS_PER_MM) as i32;

    let mut cx = start_x_steps;
    let mut cy = start_y_steps;

    let dx = (target_x_steps - start_x_steps).abs();
    let dy = -(target_y_steps - start_y_steps).abs();
    let sx = if start_x_steps < target_x_steps { 1 } else { -1 };
    let sy = if start_y_steps < target_y_steps { 1 } else { -1 };
    let mut err = dx + dy;

    while cx != target_x_steps || cy != target_y_steps {
        let e2 = 2 * err;

        if e2 >= dy {
            err += dy;
            cx += sx;
            motor_x.step_once(sx > 0);
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
            motor_y.step_once(sy > 0);
        }
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_x.stop();
    motor_y.stop();
    *current_x_mm = target_x_mm;
    *current_y_mm = target_y_mm;
}

// ==========================================
// 4. PARSARE G-CODE (Traducere Text -> Actiune)
// ==========================================
// Parsează un string (ex: "G1 X12.5 Y34.2") și caută o valoare după o anumită literă (ex: 'X').
fn get_value(text: &str, prefix: char) -> Option<f32> {
    let mut parts = text.split_whitespace();
    while let Some(part) = parts.next() {
        if part.starts_with(prefix) {
            // Tăiem prima literă și parsam restul (ex: din "X12.5" obținem 12.5)
            if let Ok(val) = part[1..].parse::<f32>() {
                return Some(val);
            }
        }
    }
    None
}

fn parse_gcode(line: &str, current_x: f32, current_y: f32) -> Command {
    // Curățăm textul de posibile goluri
    let line = line.trim();

    if line.starts_with("M3") || line.starts_with("M03") {
        return Command::PenDown;
    } else if line.starts_with("M5") || line.starts_with("M05") {
        return Command::PenUp;
    } else if line.starts_with("G0") || line.starts_with("G1") {
        // Dacă nu se trimite o nouă coordonată X, înseamnă că X-ul rămâne la fel
        let target_x = get_value(line, 'X').unwrap_or(current_x);
        let target_y = get_value(line, 'Y').unwrap_or(current_y);
        return Command::MoveTo(target_x, target_y);
    }
    
    // Ignorăm comentariile, comenzile F (Feedrate) sau Z brut etc.
    Command::Unknown
}

// ==========================================
// BUCĂLA PRINCIPALĂ (MAIN)
// ==========================================
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Sistem CNC Initializat. Asteptam comenzi via USB/UART...");

    // Inițializare Motoare
    let mut motor_z = StepperMotor::new(
        Output::new(p.PB10, Level::Low, Speed::Low),
        Output::new(p.PB4, Level::Low, Speed::Low),
        Output::new(p.PB5, Level::Low, Speed::Low),
        Output::new(p.PA8, Level::Low, Speed::Low),
    );
    let mut motor_y = StepperMotor::new(
        Output::new(p.PB3, Level::Low, Speed::Low),
        Output::new(p.PC8, Level::Low, Speed::Low),
        Output::new(p.PA2, Level::Low, Speed::Low),
        Output::new(p.PA3, Level::Low, Speed::Low),
    );
    let mut motor_x = StepperMotor::new(
        Output::new(p.PA7, Level::Low, Speed::Low),
        Output::new(p.PC9, Level::Low, Speed::Low),
        Output::new(p.PC6, Level::Low, Speed::Low),
        Output::new(p.PC7, Level::Low, Speed::Low),
    );

    // Inițializare USART1 (Comunicarea cu PC-ul prin portul USB ST-LINK)
    let mut config = Config::default();
    config.baudrate = 115200;
    let mut usart = Uart::new(p.USART1, p.PA10, p.PA9, Irqs, p.GPDMA1_CH0, p.GPDMA1_CH1, config).unwrap();
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut buf = [0u8; 128]; // Buffer pentru a citi caracterele venite de la PC

    loop {
        match usart.read_until_idle(&mut buf).await {
            Ok(len) if len > 0 => {
                // Folosim un from_utf8 sigur. Daca apar caractere ciudate/bruiaj, nu crapa, ci devine gol ("")
                let safe_str = core::str::from_utf8(&buf[..len]).unwrap_or("");
                let cmd = parse_gcode(safe_str, current_x, current_y);

                match cmd {
                    Command::PenUp => pen_up(&mut motor_z).await,
                    Command::PenDown => pen_down(&mut motor_z).await,
                    Command::MoveTo(tx, ty) => {
                        draw_line(&mut motor_x, &mut motor_y, &mut current_x, &mut current_y, tx, ty).await;
                    }
                    Command::Unknown => {
                        // Ignoram elegant comenzile necunoscute
                    }
                }

                // Indiferent ce s-a executat, trimitem OK ca sa deblocam Python-ul
                let _ = usart.write(b"OK\n").await;
            }
            Ok(_) => {
                // len == 0, nu facem nimic
            }
            Err(_) => {
                // Daca pica bufferul (Overrun Error), STM-ul a pierdut comanda.
                // TOTUSI, ii trimitem Python-ului un OK ca sa continue cu urmatoarea 
                // comanda si sa nu se blocheze la infinit.
                error!("Eroare UART! O comanda s-a pierdut.");
                let _ = usart.write(b"OK\n").await;
            }
        }
    }
}