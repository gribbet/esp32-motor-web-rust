use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_net::{Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::peripherals::{RADIO_CLK, RNG, TIMG0, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_wifi::wifi::{
    self, ClientConfiguration, Configuration, WifiDevice, WifiError, WifiStaDevice,
};
use esp_wifi::EspWifiController;
use picoserve::make_static;
use rand_core::RngCore;

pub async fn create_stack<'d>(
    spawner: Spawner,
    timg0: TIMG0,
    rng: RNG,
    radio_clk: RADIO_CLK,
    wifi: WIFI,
) -> Result<Stack<'d>, WifiError> {
    let timg0 = TimerGroup::new(timg0);
    let mut rng = Rng::new(rng);

    let (wifi, mut controller) = wifi::new_with_mode(
        make_static!(
            EspWifiController<'_>,
            esp_wifi::init(timg0.timer0, rng, radio_clk).unwrap()
        ),
        wifi,
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

    Ok(stack)
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}
