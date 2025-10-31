#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    info!("Board initialized!");

    // Servo control pin on TIM2_CH1 (PA0)
    let servo_pin = PwmPin::new_ch1(p.PA0, OutputType::PushPull);

    // 50 Hz PWM (20 ms period)
    let mut pwm = SimplePwm::new(
        p.TIM2,
        Some(servo_pin),
        None,
        None,
        None,
        hz(50),
        Default::default(),
    );

    let mut ch1 = pwm.ch1();
    ch1.enable();

    // Duty values for 1ms–2ms pulses
    let min_duty = 1200; // 0 degrees
    let max_duty = 7500; // 180 degrees

    loop {
        // Sweep forward
        for duty in min_duty..=max_duty {
            ch1.set_duty_cycle(duty as u16);
            Timer::after_millis(5).await;
        }

        // Sweep backward
        for duty in (min_duty..=max_duty).rev() {
            ch1.set_duty_cycle(duty as u16);
            Timer::after_millis(5).await;
        }
    }
}
