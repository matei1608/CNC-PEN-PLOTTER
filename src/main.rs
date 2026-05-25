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
// 2. CALIBRAREA (FACTORUL DE CONVERSIE)
// ==========================================
const STEPS_PER_REV: f32 = 4096.0; // Nr. de pași pentru o rotație completă (Half-Step)
// Atenție: 32.0 este o valoare estimativă. Reprezintă câți mm se mișcă axa la o rotație completă a rotiței.
const PINION_CIRCUMFERENCE_MM: f32 = 48.0; 
const STEPS_PER_MM: f32 = STEPS_PER_REV / PINION_CIRCUMFERENCE_MM;

// Secvența Half-Step pentru motorul 28BYJ-48
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
// STRUCTURĂ NOUĂ PENTRU CONTROLUL MOTOARELOR
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

    // Funcția cheie care face EXACT UN PAS.
    // Va fi folosită intens de Algoritmul Bresenham în pasul următor.
    fn step_once(&mut self, forward: bool) {
        if forward {
            self.step_index = (self.step_index + 1) % 8;
        } else {
            self.step_index = (self.step_index + 7) % 8; // Matematic echivalent cu -1 pentru array
        }

        let step_state = STEP_SEQUENCE[self.step_index];
        if step_state[0] { self.in1.set_high(); } else { self.in1.set_low(); }
        if step_state[1] { self.in2.set_high(); } else { self.in2.set_low(); }
        if step_state[2] { self.in3.set_high(); } else { self.in3.set_low(); }
        if step_state[3] { self.in4.set_high(); } else { self.in4.set_low(); }
    }
    // Funcție nouă pentru a tăia curentul (să nu se încălzească motorul degeaba)
    fn stop(&mut self) {
        self.in1.set_low();
        self.in2.set_low();
        self.in3.set_low();
        self.in4.set_low();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    
    info!("Sistem CNC initializat!");
    info!("Calibrare: {} pasi pentru 1 milimetru.", STEPS_PER_MM);

    // Initializare Motoare cu pinii tai corectati
    let _motor_z = StepperMotor::new(
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
    // G-CODE HARDCODAT (Traseul nostru de test)
    // Va desena o linie de 10mm, apoi una de 20mm.
    // ==========================================
    let traseu_test = [
        Command::PenUp,
        Command::MoveTo(10.0, 10.0), // Ne deplasam in pozitia de start (10mm, 10mm)
        Command::PenDown,
        Command::MoveTo(20.0, 10.0), // Desenam o linie orizontala de 10mm
        Command::MoveTo(20.0, 30.0), // Desenam o linie verticala de 20mm
        Command::PenUp,
        Command::MoveTo(0.0, 0.0),   // Ne intoarcem la punctul de Origine (Home)
    ];

    info!("Incepem executia traseului hardcodat...");

    // Aici va veni bucla care parcurge array-ul `traseu_test` 
    // si apeleaza functiile de miscare pe care le vom scrie la Pasul 3 si 4.

    // Rulăm fix 4096 de pași
    for _ in 0..4096 {
        motor_y.step_once(true); // true = direcția înainte. Pune 'false' dacă vrei să se miște invers.
        Timer::after(Duration::from_micros(1500)).await; // 1.5ms delay între pași pentru mișcare fină
    }

    

    info!("GATA! Masoara acum cati milimetri s-a deplasat cremaliera.");
    
    // Oprim curentul de la motor
    motor_y.stop();


    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}