#![no_std]
#![no_main]

extern crate alloc;

use blocking_network_stack::Stack;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{main, time, Config};
use esp_println::println;
use esp_wifi::wifi::utils::create_network_interface;
use esp_wifi::wifi::{ClientConfiguration, Configuration, WifiError, WifiStaDevice};
use smoltcp::iface::{SocketSet, SocketStorage};
use smoltcp::socket::dhcpv4;

const SSID: &str = "Wokwi-GUEST";
const PASSWORD: &str = "";

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    esp_alloc::heap_allocator!(72 * 1024);

    run().unwrap_or_else(|error| panic!("{:?}", error));

    loop {}
}

fn run() -> Result<(), WifiError> {
    let peripherals = esp_hal::init(Config::default().with_cpu_clock(CpuClock::max()));

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let mut rng = Rng::new(peripherals.RNG);

    let esp_controller = esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK).unwrap();

    let (interface, device, mut controller) =
        create_network_interface(&esp_controller, peripherals.WIFI, WifiStaDevice)?;

    let mut socket_set_entries: [SocketStorage; 1] = Default::default();
    let mut socket_set = SocketSet::new(&mut socket_set_entries[..]);
    socket_set.add(dhcpv4::Socket::new());

    let now = || time::now().duration_since_epoch().to_millis();
    let stack = Stack::new(interface, device, socket_set, now, rng.random());

    controller.start()?;

    println!("Scanning");
    let (access_points, _) = controller.scan_n::<100>()?;
    for access_point in access_points {
        println!("{}", access_point.ssid);
    }

    controller.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    }))?;
    controller.connect()?;

    println!("Waiting for IP address");
    loop {
        stack.work();

        if stack.is_iface_up() {
            println!("IP: {:?}", stack.get_ip_info());
            break;
        }
    }

    Ok(())
}
