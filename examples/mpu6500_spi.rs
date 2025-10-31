#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::{info, unwrap};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{self, Config, Spi};
use embassy_stm32::time::Hertz;
use {defmt_rtt as _, panic_probe as _};

#[entry]
fn main() -> ! {
    info!("Board started!");

    let p = embassy_stm32::init(Default::default());

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
    let mut cs = Output::new(p.PC9, Level::High, Speed::VeryHigh);

    loop {
        let tx_buf: [u8; 1] = [0x3B | 0x80]; // Read command
        let mut rx_buf = [0u8; 14]; // Receive buffer

        cs.set_low();
        spi.blocking_write(&tx_buf).unwrap(); // Send read command
        spi.blocking_read(&mut rx_buf).unwrap(); // Read response
        cs.set_high();

        // Convert raw data
        let ax_raw = ((rx_buf[0] as i16) << 8 | (rx_buf[1] as i16)) as f32;
        let ay_raw = ((rx_buf[2] as i16) << 8 | (rx_buf[3] as i16)) as f32;
        let az_raw = ((rx_buf[4] as i16) << 8 | (rx_buf[5] as i16)) as f32;

        let gx_raw = ((rx_buf[8] as i16) << 8 | (rx_buf[9] as i16)) as f32;
        let gy_raw = ((rx_buf[10] as i16) << 8 | (rx_buf[11] as i16)) as f32;
        let gz_raw = ((rx_buf[12] as i16) << 8 | (rx_buf[13] as i16)) as f32;

        // Apply scaling
        let accel_scale = 9.81 / 16384.0;
        let gyro_scale = 250.0 / 32768.0;

        let ax = ax_raw * accel_scale;
        let ay = ay_raw * accel_scale;
        let az = az_raw * accel_scale;

        let gx = gx_raw * gyro_scale;
        let gy = gy_raw * gyro_scale;
        let gz = gz_raw * gyro_scale;

        info!(
            "Acceleration (m/s²): X = {=f32}, Y = {=f32}, Z = {=f32}",
            ax, ay, az
        );
        info!(
            "Gyroscope (°/s): X = {=f32}, Y = {=f32}, Z = {=f32}",
            gx, gy, gz
        );
    }
}
