#![no_std]
#![no_main]

// Example originally designed for stm32f411ceu6 reading an A1454 hall effect sensor on I2C1
// DMA peripherals changed to compile for stm32f429zi, for the CI.

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::i2c::I2c;
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

const BMP280_ADDR: u8 = 0x76; // Default address (0x77 if SDO is high)
const REG_ID: u8 = 0xD0;
const REG_RESET: u8 = 0xE0;
const REG_CTRL_MEAS: u8 = 0xF4;
const REG_CONFIG: u8 = 0xF5;
const REG_TEMP: u8 = 0xFA;

const DIG_T1: u16 = 27504;
const DIG_T2: i16 = 26435;
const DIG_T3: i16 = -1000;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello world!");
    let p = embassy_stm32::init(Default::default());

    let mut i2c = I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        Irqs,
        p.GPDMA1_CH0,
        p.GPDMA1_CH1,
        Hertz(100_000),
        Default::default(),
    );

    let mut id = [0u8; 1];
    if i2c
        .blocking_write_read(BMP280_ADDR, &[REG_ID], &mut id)
        .is_err()
    {
        error!("BMP280 not found!");
        return;
    }
    info!("BMP280 detected with ID: 0x{:02X}", id[0]);

    // Reset the device (optional)
    let _ = i2c.blocking_write(BMP280_ADDR, &[REG_RESET, 0xB6]);

    // Configure measurement (Temp oversampling x2, Normal mode)
    // 0b010_000_11 = 0x43
    if i2c
        .blocking_write(BMP280_ADDR, &[REG_CTRL_MEAS, 0x43])
        .is_err()
    {
        error!("Failed to configure BMP280!");
        return;
    }

    // Configuration register (optional settings)
    let _ = i2c.blocking_write(BMP280_ADDR, &[REG_CONFIG, 0b000_00_000]);

    info!("BMP280 configured. Starting temperature readings...");

    loop {
        // Read raw temperature (3 bytes: MSB, LSB, XLSB)
        let mut temp_data = [0u8; 3];
        if i2c
            .blocking_write_read(BMP280_ADDR, &[REG_TEMP], &mut temp_data)
            .is_err()
        {
            error!("Failed to read temperature!");
        } else {
            // Combine the 3 bytes into a 20-bit raw value
            let raw_temp = ((temp_data[0] as u32) << 12)
                | ((temp_data[1] as u32) << 4)
                | ((temp_data[2] as u32) >> 4);

            // Calculate actual temperature using compensation formula
            let actual_temp = compensate_temperature(raw_temp);

            // Print temperature in XX.YY°C format
            info!(
                "Temperature {}.{:02}°C",
                actual_temp / 100,
                actual_temp.abs() % 100
            );
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}

fn compensate_temperature(raw_temp: u32) -> i32 {
    // Temperature compensation formula from BMP280 datasheet
    let var1 = ((((raw_temp as i32) >> 3) - ((DIG_T1 as i32) << 1)) * (DIG_T2 as i32)) >> 11;
    let var2 = (((((raw_temp as i32) >> 4) - (DIG_T1 as i32))
        * (((raw_temp as i32) >> 4) - (DIG_T1 as i32)))
        >> 12)
        * (DIG_T3 as i32)
        >> 14;
    let t_fine = var1 + var2;
    let temp = (t_fine * 5 + 128) >> 8;

    temp // Returns temperature in °C * 100 (2425 = 24.25°C)
}
