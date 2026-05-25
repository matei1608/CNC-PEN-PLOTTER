#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// Secvența Half-Step pentru motorul 28BYJ-48 (8 pași)
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

// Definim un Task Embassy refolosibil pentru fiecare motor
// Adăugăm parametrul `forward: bool`
#[embassy_executor::task(pool_size = 3)]
async fn stepper_task(
    mut in1: Output<'static>,
    mut in2: Output<'static>,
    mut in3: Output<'static>,
    mut in4: Output<'static>,
    axis_name: &'static str,
    delay_micros: u64,
    forward: bool, // <-- Parametru NOU
) {
    let step_delay = embassy_time::Duration::from_micros(delay_micros);
    let directie_str = if forward { "Inainte" } else { "Inapoi" };
    info!("Task-ul pentru axa {} a pornit! Directie: {}", axis_name, directie_str);

    loop {
        if forward {
            // Merge înainte (de la 0 la 7)
            for step in STEP_SEQUENCE.iter() {
                if step[0] { in1.set_high(); } else { in1.set_low(); }
                if step[1] { in2.set_high(); } else { in2.set_low(); }
                if step[2] { in3.set_high(); } else { in3.set_low(); }
                if step[3] { in4.set_high(); } else { in4.set_low(); }
                Timer::after(step_delay).await;
            }
        } else {
            // Merge înapoi folosind .rev() (de la 7 la 0)
            for step in STEP_SEQUENCE.iter().rev() {
                if step[0] { in1.set_high(); } else { in1.set_low(); }
                if step[1] { in2.set_high(); } else { in2.set_low(); }
                if step[2] { in3.set_high(); } else { in3.set_low(); }
                if step[3] { in4.set_high(); } else { in4.set_low(); }
                Timer::after(step_delay).await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Sistem CNC pornit. Inițializăm ambele axe...");

    // === CONFIGURARE AXA Z ===
    let in4_z = Output::new(p.PA8, Level::Low, Speed::Low); // D7
    let in3_z = Output::new(p.PB5, Level::Low, Speed::Low); // D4
    let in2_z = Output::new(p.PB4, Level::Low, Speed::Low); // D5
    let in1_z = Output::new(p.PB10, Level::Low, Speed::Low); // D11

    // === CONFIGURARE AXA Y ===
    let in1_y = Output::new(p.PB3, Level::Low, Speed::Low);  // D3
    let in2_y = Output::new(p.PC8, Level::Low, Speed::Low); // D2
    let in3_y = Output::new(p.PA2, Level::Low, Speed::Low);  // D1
    let in4_y = Output::new(p.PA3, Level::Low, Speed::Low);  // D0

     // === CONFIGURARE AXA X ===
    let in4_x = Output::new(p.PC7, Level::Low, Speed::Low);  // D3
    let in3_x = Output::new(p.PC6, Level::Low, Speed::Low); // D2
    let in2_x = Output::new(p.PC9, Level::Low, Speed::Low);  // D1
    let in1_x = Output::new(p.PA7, Level::Low, Speed::Low);  // D0

    // Pornim ambele motoare în paralel folosind spawner-ul Embassy.
    // Al șaselea parametru este delay-ul (viteza). Le poți pune viteze diferite dacă vrei!
    unwrap!(spawner.spawn(stepper_task(in1_z, in2_z, in3_z, in4_z, "Z", 400, false)));
    unwrap!(spawner.spawn(stepper_task(in1_y, in2_y, in3_y, in4_y, "Y", 400, false)));
    unwrap!(spawner.spawn(stepper_task(in1_x, in2_x, in3_x, in4_x, "X", 400, false)));

    // Bucla din main nu mai trebuie să facă nimic, task-urile rulează în fundal
    loop {
        Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
}