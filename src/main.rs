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

// Constanta pentru ridicarea/coborarea creionului.
// 1000 de pași înseamnă cam un sfert de tură a rotiței Z. Ajustează valoarea asta după test!
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
        // Punem "true" pentru a merge într-o direcție. (Dacă ridicarea e invers, pui "false")
        motor_z.step_once(true); 
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_z.stop(); // Oprim curentul imediat ce a ajuns sus
}

async fn pen_down(motor_z: &mut StepperMotor) {
    info!("Executam: PEN DOWN (Coboram Creionul)");
    for _ in 0..Z_STEPS_MOVE {
        // Punem "false" ca să se învârtă exact în sensul opus față de Pen UP.
        motor_z.step_once(false);
        Timer::after(Duration::from_micros(1500)).await;
    }
    motor_z.stop();
}


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    
    info!("Sistem CNC initializat!");

    // Setăm motorul Z
    let mut motor_z = StepperMotor::new(
        Output::new(p.PB10, Level::Low, Speed::Low),
        Output::new(p.PB4, Level::Low, Speed::Low),
        Output::new(p.PB5, Level::Low, Speed::Low),
        Output::new(p.PA8, Level::Low, Speed::Low),
    );

    // X si Y sunt lasate in asteptare, nu le vom misca in acest test
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
    // TESTUL FIZIC PENTRU Z
    // ==========================================
    info!("Incepem testul pentru axa Z. Ai 3 secunde...");
    Timer::after(Duration::from_secs(3)).await;

    
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}