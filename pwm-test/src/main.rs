#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::info;
use embassy_embedded_hal::SetConfig;
use embassy_executor::Spawner;
use embassy_rp::{
    // gpio::PwmPin,
    pwm::{Config, Pwm},
};
use embassy_time::{Duration, Timer};

use {defmt_rtt as _, panic_probe as _};

mod servo {
    use core::u8;

    use cortex_m::prelude::_embedded_hal_PwmPin;
    use defmt::export::UnsignedInt;
    use embassy_rp::{
        gpio::AnyPin,
        pwm::{Config, Pwm}, Peripheral,
    };

    pub struct Servo<PIN, POS>
// where
    //     PIN: AnyPin,
    //     POS: u8,
    {
        pin: PIN,
        pos: POS, //position in degrees 0 - 180 usually
    }
    impl<PIN, POS> Servo<PIN, POS>
    where
        PIN: _embedded_hal_PwmPin,
        POS: u8
    {
        pub fn new(pin: PIN, pos: POS) {
            let mut pwm_c: Config = Default::default();
            pwm_c.top = 9884;
            pwm_c.divider = 255.into();
            pwm_c.compare_b = 1; //TODO figure this out
            pwm_c.compare_a = 1;
            let mut pwm = Pwm::new_output_b(, p.PIN_25, pwm_c.clone());
            // let mut pwm2 = Pwm::new_output_a(&p.PWM_CH0, p.PIN_16, pwm_c.clone());
        }
    }
}
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello, world!");
    let p = embassy_rp::init(Default::default());
    let mut pwm_c: Config = Default::default();
    pwm_c.top = 9884;
    pwm_c.divider = 255.into();
    pwm_c.compare_b = 1; //TODO figure this out
    pwm_c.compare_a = 1;
    let mut pwm = Pwm::new_output_b(&p.PWM_CH4, p.PIN_25, pwm_c.clone());
    let mut pwm2 = Pwm::new_output_a(&p.PWM_CH0, p.PIN_16, pwm_c.clone());

    loop {
        info!("current LED duty cycle: {}/32768", pwm_c.compare_b);
        Timer::after(Duration::from_secs(1)).await;
        pwm_c.compare_b += 10;
        pwm_c.compare_a += 10;
        pwm.set_config(&pwm_c);
        pwm2.set_config(&pwm_c);
    }
}
