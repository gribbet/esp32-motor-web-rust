use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::ledc::channel::config::PinConfig;
use esp_hal::ledc::channel::{Channel, ChannelIFace};
use esp_hal::ledc::timer::config::Duty;
use esp_hal::ledc::timer::{LSClockSource, Timer, TimerIFace};
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::peripheral::Peripheral;
use esp_hal::peripherals::LEDC;
use esp_hal::time::RateExtU32 as _;

pub struct LedController<'d> {
    ledc: Ledc<'d>,
    timer: Timer<'d, LowSpeed>,
}

impl<'d> LedController<'d> {
    pub fn new(ledc: LEDC) -> Self {
        let mut ledc = Ledc::new(ledc);
        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
        let mut timer = ledc.timer(timer::Number::Timer0);
        timer
            .configure(timer::config::Config {
                duty: Duty::Duty8Bit,
                clock_source: LSClockSource::APBClk,
                frequency: 24u32.kHz(),
            })
            .unwrap();
        Self { ledc, timer }
    }

    pub fn led(&'d self, output: impl Peripheral<P = impl PeripheralOutput> + 'd) -> Led<'d> {
        Led::new(self, output)
    }
}

pub struct Led<'d> {
    value: u8,
    channel: Channel<'d, LowSpeed>,
}

impl<'d> Led<'d> {
    fn new(
        controller: &'d LedController<'d>,
        output: impl Peripheral<P = impl PeripheralOutput> + 'd,
    ) -> Self {
        let mut channel = controller.ledc.channel(channel::Number::Channel0, output);
        channel
            .configure(channel::config::Config {
                timer: &controller.timer,
                duty_pct: 100,
                pin_config: PinConfig::PushPull,
            })
            .unwrap();
        Self { channel, value: 0 }
    }

    pub fn get_brightness(&self) -> u8 {
        self.value
    }

    pub fn set_brightness(&mut self, value: u8) {
        self.channel.set_duty(100 - value).unwrap();
        self.value = value;
    }
}
