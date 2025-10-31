#![no_std]
#![no_main]

use core::fmt::Write;
use core::str::from_utf8;

use defmt::*;
use embassy_executor::Spawner;
// use embassy_stm32::lptim::timer::Timer;
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::Timer;
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);

    let mut spi = Spi::new(
        p.SPI1,
        p.PA5,
        p.PA7,
        p.PA6,
        p.GPDMA1_CH0,
        p.GPDMA1_CH1,
        spi_config,
    );

    for n in 0u32.. {
        let mut write: String<128> = String::new();
        let mut read = [0; 128];
        core::write!(&mut write, "Hello DMA World {}!\r\n", n).unwrap();
        spi.transfer(&mut read[0..write.len()], write.as_bytes())
            .await
            .ok();
        Timer::after_millis(500).await;
        info!("read via spi+dma: {}", &read);
    }
}
