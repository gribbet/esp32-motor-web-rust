#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::rng::Rng;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::Config;
use esp_println::println;
use esp_wifi::wifi::{ClientConfiguration, Configuration, WifiDevice, WifiError, WifiStaDevice};
use esp_wifi::EspWifiController;
use rand_core::RngCore;
use static_cell::StaticCell;

macro_rules! make_static {
    ($t:ty, $val:expr) => {{
        static CELL: StaticCell<$t> = StaticCell::new();
        CELL.uninit().write($val)
    }};
}

const SSID: &str = "Wokwi-Guest";
const PASSWORD: &str = "";

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    esp_alloc::heap_allocator!(72 * 1024);

    run(spawner)
        .await
        .unwrap_or_else(|error| panic!("{:?}", error));
}

async fn run(spawner: Spawner) -> Result<(), WifiError> {
    let peripherals = esp_hal::init(Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let init = make_static!(
        EspWifiController<'_>,
        esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK).unwrap()
    );

    let (wifi_interface, mut controller) =
        esp_wifi::wifi::new_with_mode(init, peripherals.WIFI, WifiStaDevice)?;

    esp_hal_embassy::init(SystemTimer::new(peripherals.SYSTIMER).alarm0);

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        embassy_net::Config::dhcpv4(Default::default()),
        make_static!(StackResources<3>, StackResources::new()),
        rng.next_u64(),
    );

    spawner.spawn(net_task(runner)).unwrap();

    controller.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    }))?;

    println!("Starting wifi");
    controller.start_async().await?;
    println!("Wifi started!");

    println!("Scanning");
    let (access_points, _) = controller.scan_n::<100>()?;
    for access_point in access_points {
        println!("{}", access_point.ssid);
    }

    match controller.connect_async().await {
        Ok(_) => {
            println!("Wifi connected!");
            while !stack.is_link_up() {
                Timer::after(Duration::from_millis(500)).await;
            }

            println!("Waiting to get IP address...");
            loop {
                if let Some(config) = stack.config_v4() {
                    println!("Got IP: {}", config.address);
                    break;
                }
                Timer::after(Duration::from_millis(100)).await;
            }
        }
        Err(e) => println!("{e:?}"),
    }

    controller.stop_async().await?;

    Ok(())
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}
