#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::Config;
use led::LedController;
use net::create_stack;
use picoserve::make_static;
use server::start_server;

mod led;
mod net;
mod server;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    esp_alloc::heap_allocator!(72 * 1024);

    let peripherals = esp_hal::init(Config::default().with_cpu_clock(CpuClock::max()));

    esp_hal_embassy::init(SystemTimer::new(peripherals.SYSTIMER).alarm0);

    let stack = create_stack(
        spawner,
        peripherals.TIMG0,
        peripherals.RNG,
        peripherals.RADIO_CLK,
        peripherals.WIFI,
    )
    .await
    .unwrap_or_else(|error| panic!("{:?}", error));

    let controller = make_static!(LedController, LedController::new(peripherals.LEDC));
    let led = controller.led(peripherals.GPIO7);

    start_server(spawner, stack, led).await;
}
