#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

// ==========================================
// 1. DEFINIREA COMENZILOR G-CODE HARDCODATE
// ==========================================
#[derive(Clone, Copy)]
enum Command {
    PenUp,
    PenDown,
    MoveTo(f32, f32), // Coordonate X și Y în milimetri
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
// 3. CONTROLUL AXEI Z
// ==========================================
async fn pen_up(motor_z: &mut StepperMotor) {
    info!("Executam: PEN UP (Ridicam Creionul)");
    for _ in 0..Z_STEPS_MOVE {
        motor_z.step_once(true); 
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_z.stop();
}

async fn pen_down(motor_z: &mut StepperMotor) {
    info!("Executam: PEN DOWN (Coboram Creionul)");
    for _ in 0..Z_STEPS_MOVE {
        motor_z.step_once(false);
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_z.stop();
}

// ==========================================
// 4. ALGORITMUL LUI BRESENHAM (CREIERUL CNC)
// ==========================================
async fn draw_line(
    motor_x: &mut StepperMotor,
    motor_y: &mut StepperMotor,
    current_x_mm: &mut f32,
    current_y_mm: &mut f32,
    target_x_mm: f32,
    target_y_mm: f32,
) {
    info!("Trasam linie de la ({}, {}) la ({}, {})", *current_x_mm, *current_y_mm, target_x_mm, target_y_mm);

    // 1. Convertim coordonatele din milimetri in numar de pasi absoluti
    let start_x_steps = (*current_x_mm * STEPS_PER_MM) as i32;
    let start_y_steps = (*current_y_mm * STEPS_PER_MM) as i32;
    let target_x_steps = (target_x_mm * STEPS_PER_MM) as i32;
    let target_y_steps = (target_y_mm * STEPS_PER_MM) as i32;

    let mut cx = start_x_steps;
    let mut cy = start_y_steps;

    // 2. Setup pentru Bresenham
    let dx = (target_x_steps - start_x_steps).abs();
    let dy = -(target_y_steps - start_y_steps).abs();
    let sx = if start_x_steps < target_x_steps { 1 } else { -1 };
    let sy = if start_y_steps < target_y_steps { 1 } else { -1 };
    let mut err = dx + dy;

    // 3. Bucla de miscare sincronizata
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

        // Delay-ul pentru a controla viteza de deplasare a capului
        Timer::after(Duration::from_micros(1500)).await;
    }

    // Oprim curentul la ambele motoare dupa ce au ajuns la destinatie
    motor_x.stop();
    motor_y.stop();

    // Actualizam memoria sistemului cu noua pozitie
    *current_x_mm = target_x_mm;
    *current_y_mm = target_y_mm;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    
    info!("Sistem CNC initializat! Pasi pe mm: {}", STEPS_PER_MM);

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

    // ==========================================
    // TRASEUL HARDCODAT (Pătrat 50x50 mm)
    // ==========================================
    let traseu_test = [
        // Ne asigurăm că suntem fix la Origine (0,0)
        
        // --- 1. DESENAREA PĂTRATULUI ---
        Command::PenDown,             // Punem pixul pe foaie
        Command::MoveTo(50.0, 0.0),   // Desenăm latura de jos (spre dreapta)
        Command::MoveTo(50.0, 50.0),  // Desenăm latura din dreapta (în sus)
        Command::MoveTo(0.0, 50.0),   // Desenăm latura de sus (spre stânga)
        Command::MoveTo(0.0, 0.0),    // Desenăm latura din stânga (în jos) - pătrat închis
        
        // --- 2. PRIMA DIAGONALĂ ---
        // Acum suntem la (0,0) și pixul este deja pe foaie.
        Command::MoveTo(50.0, 50.0),  // Tragem diagonala din Stânga-Jos spre Dreapta-Sus
        
        // --- 3. A DOUA DIAGONALĂ ---
        Command::PenUp,               // Ridicăm pixul ca să nu mâzgălim desenul
        Command::MoveTo(0.0, 50.0),   // Ne mutăm „în aer” în colțul din Stânga-Sus
        Command::PenDown,             // Lăsăm pixul înapoi pe foaie
        Command::MoveTo(50.0, 0.0),   // Tragem diagonala din Stânga-Sus spre Dreapta-Jos
        
        // --- 4. FINALIZARE ---
        Command::PenUp,               // Ridicăm pixul de pe foaie la final
        Command::MoveTo(0.0, 0.0),    // Ne întoarcem la punctul de start (Acasă)
    ];

    info!("Ai 2 secunde sa pozitionezi creionul manual in coltul din Stanga-Jos al foii...");
    Timer::after(Duration::from_secs(2)).await;

    // Memoria sistemului (plecam din coordonatele X=0, Y=0)
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    info!("Incepem desenarea patratului!");

    // Parcurgem lista de comenzi
    for cmd in traseu_test.iter() {
        match cmd {
            Command::PenUp => {
                pen_up(&mut motor_z).await;
            }
            Command::PenDown => {
                pen_down(&mut motor_z).await;
            }
            Command::MoveTo(target_x, target_y) => {
                draw_line(
                    &mut motor_x, 
                    &mut motor_y, 
                    &mut current_x, 
                    &mut current_y, 
                    *target_x, 
                    *target_y
                ).await;
            }
        }
        // O pauza scurta dupa fiecare comanda pentru a lasa mecanismul sa se stabilizeze
        Timer::after(Duration::from_millis(300)).await;
    }

    info!("DESEN FINALIZAT! Motoarele sunt oprite.");

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}