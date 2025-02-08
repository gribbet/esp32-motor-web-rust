#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::Config;
use led::LedController;
use server::start;

mod led;
mod server;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    esp_alloc::heap_allocator!(72 * 1024);

    let peripherals = esp_hal::init(Config::default().with_cpu_clock(CpuClock::max()));

    esp_hal_embassy::init(SystemTimer::new(peripherals.SYSTIMER).alarm0);

    start(
        spawner,
        peripherals.TIMG0,
        peripherals.RNG,
        peripherals.RADIO_CLK,
        peripherals.WIFI,
    )
    .await
    .unwrap_or_else(|error| panic!("{:?}", error));

    let controller = LedController::new(peripherals.LEDC);
    let led = controller.led(peripherals.GPIO7);

    let mut i = 0u32;
    while i < 100000 {
        led.set((i % 100) as u8);
        i += 1;
        Timer::after(Duration::from_millis(5)).await;
    }
}
