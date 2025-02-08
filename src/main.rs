#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_net::Stack;
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
use esp_wifi::wifi::{
    self, ClientConfiguration, Configuration, WifiDevice, WifiError, WifiStaDevice,
};
use esp_wifi::EspWifiController;
use led::LedController;
use picoserve::routing::get;
use picoserve::AppBuilder;
use picoserve::{make_static, AppRouter};
use rand_core::RngCore;

mod led;

const WEB_TASK_POOL_SIZE: usize = 8;

struct AppProps {
    message: &'static str,
}

impl AppBuilder for AppProps {
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        let Self { message } = self;

        picoserve::Router::new().route("/", get(move || async move { message }))
    }
}

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

    esp_hal_embassy::init(SystemTimer::new(peripherals.SYSTIMER).alarm0);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let (wifi, mut controller) = wifi::new_with_mode(
        make_static!(
            EspWifiController<'_>,
            esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK).unwrap()
        ),
        peripherals.WIFI,
        WifiStaDevice,
    )?;

    let (stack, runner) = embassy_net::new(
        wifi,
        embassy_net::Config::dhcpv4(Default::default()),
        make_static!(StackResources<20>, StackResources::new()),
        rng.next_u64(),
    );

    spawner.must_spawn(net_task(runner));

    controller.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: "Wokwi-GUEST".try_into().unwrap(),
        auth_method: wifi::AuthMethod::None,
        ..Default::default()
    }))?;

    println!("Starting WiFi...");
    controller.start_async().await?;

    println!("Connecting...");
    while controller.connect_async().await.is_err() {
        Timer::after(Duration::from_millis(5)).await;
    }

    println!("Waiting for IP...");
    stack.wait_config_up().await;

    let address = stack.config_v4().unwrap().address;
    println!("IP: {}", address);

    let config = make_static!(
        picoserve::Config::<Duration>,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    let app = make_static!(
        AppRouter<AppProps>,
        AppProps {
            message: "Hello World"
        }
        .build_app()
    );

    for id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(id, stack, app, config));
    }

    let controller = LedController::new(peripherals.LEDC);
    let led = controller.led(peripherals.GPIO7);

    let mut i = 0u32;
    while i < 100000 {
        led.set((i % 100) as u8);
        i += 1;
        Timer::after(Duration::from_millis(5)).await;
    }

    Ok(())
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: Stack<'static>,
    app: &'static AppRouter<AppProps>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    )
    .await
}
