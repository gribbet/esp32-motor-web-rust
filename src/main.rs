#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output},
    main,
};
use esp_println::println;

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    println!("Hello world!");

    let mut led = Output::new(peripherals.GPIO7, Level::High);

    let delay = Delay::new();

    loop {
        led.toggle();
        delay.delay_millis(500);
    }
}
