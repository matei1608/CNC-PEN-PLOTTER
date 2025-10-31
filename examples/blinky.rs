#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());

    let mut led = Output::new(p.PA5, Level::Low, Speed::Medium); //The builtin LED of the board(LD2) is on the pin PA5.

    loop {
        defmt::info!("on!");
        led.set_low();
        Timer::after_millis(200).await;

        defmt::info!("off!");
        led.set_high();
        Timer::after_millis(200).await;
    }
}
