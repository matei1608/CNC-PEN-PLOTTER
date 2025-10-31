#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::lptim::pwm::Pwm;
use embassy_stm32::time::khz;
use embassy_stm32::timer::Channel as PwmChannel;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    info!("Board initialized!");

    // Specify the timer peripheral, e.g., TIM2
    let led_pwm_pin: PwmPin<
        '_,
        embassy_stm32::peripherals::TIM2,
        embassy_stm32::timer::simple_pwm::Ch1,
    > = PwmPin::new_ch1(p.PA0, OutputType::PushPull);

    let mut pwm_led = SimplePwm::new(
        p.TIM2,
        Some(led_pwm_pin),
        None,
        None,
        None,
        khz(1),
        Default::default(), // Set PWM frequency to 1 kHz
    );

    let mut ch1 = pwm_led.ch1();
    ch1.enable();

    loop {
        ch1.set_duty_cycle(10); // Set duty cycle to 10%
    }
}
