#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Config};
use embassy_rp::i2c::{self, Config as i2cConfig};
use embassy_rp::interrupt;
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;
use ssd1306::{mode::TerminalMode, prelude::*, I2CDisplayInterface, Ssd1306};

use core::fmt::Write;
use {defmt_rtt as _, panic_probe as _};

fn convert_to_celsius(raw_temp: u16) -> f32 {
    // According to chapter 4.9.5. Temperature Sensor in RP2040 datasheet
    27.0 - (raw_temp as f32 * (3.23 / 4096.0) - 0.706) / 0.001721 as f32
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello, world!");
    let p = embassy_rp::init(Default::default());
    let irq = interrupt::take!(ADC_IRQ_FIFO);
    let mut adc = Adc::new(p.ADC, irq, Config::default());
    info!("set up i2c ");
    let sda = p.PIN_10;
    let scl = p.PIN_11;
    let i2cirq = interrupt::take!(I2C1_IRQ);
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, i2cirq, i2cConfig::default());
    let interface = I2CDisplayInterface::new(i2c);
    let mut display =
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0).into_terminal_mode();
    display.init().unwrap();
    display.clear().unwrap();
    loop {
        let temp = adc.blocking_read_temperature();
        info!("Temp: {} degrees", convert_to_celsius(temp));
        //        let s = Formattedmat!("{:.2}", convert_to_);
        write!(display, "temp {:.1}\n", convert_to_celsius(temp)).unwrap(); //display.write_str(convert_to_celsius(temp));
        Timer::after(Duration::from_secs(1)).await;
    }
}
