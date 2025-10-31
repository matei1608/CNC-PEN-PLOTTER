#![no_std]
#![no_main]

use defmt::info;
use embassy_stm32::adc;
use embassy_stm32::adc::{AdcChannel, adc4};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let p = embassy_stm32::init(Default::default());
    let mut adc_pin = p.PA0;

    info!("Board initialized!");

    let mut adc = adc::Adc::new(p.ADC1);
    adc.set_resolution(adc::Resolution::BITS14);
    adc.set_averaging(adc::Averaging::Samples1024);
    adc.set_sample_time(adc::SampleTime::CYCLES160_5);

    loop {
        let raw: u16 = adc.blocking_read(&mut adc_pin);
        let voltage: f32 = 3.3 * raw as f32 / 16383.0; // Convert raw value to voltage
        info!("Read adc value {}", voltage);
    }
}
