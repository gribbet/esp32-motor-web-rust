use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::ledc::channel::config::PinConfig;
use esp_hal::ledc::channel::{Channel, ChannelIFace, Error};
use esp_hal::ledc::timer::config::Duty;
use esp_hal::ledc::timer::LSClockSource;
use esp_hal::ledc::timer::{Timer, TimerIFace};
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::peripheral::Peripheral;
use esp_hal::peripherals::LEDC;
use esp_hal::time::RateExtU32 as _;

pub struct MotorController<'d> {
    ledc: Ledc<'d>,
    timer: Timer<'d, LowSpeed>,
}

impl<'d> MotorController<'d> {
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
        MotorController { ledc, timer }
    }

    pub fn motor(
        &'d self,
        output1: impl Peripheral<P = impl PeripheralOutput> + 'd,
        output2: impl Peripheral<P = impl PeripheralOutput> + 'd,
    ) -> Motor<'d> {
        let channel1 = self.channel(channel::Number::Channel0, output1);
        let channel2 = self.channel(channel::Number::Channel1, output2);

        Motor::new(channel1, channel2)
    }

    fn channel(
        &'d self,
        channel: channel::Number,
        output: impl Peripheral<P = impl PeripheralOutput> + 'd,
    ) -> Channel<'d, LowSpeed> {
        let mut channel = self.ledc.channel(channel, output);
        channel
            .configure(channel::config::Config {
                timer: &self.timer,
                duty_pct: 0,
                pin_config: PinConfig::PushPull,
            })
            .unwrap();
        channel
    }
}

pub struct Motor<'d> {
    channel1: Channel<'d, LowSpeed>,
    channel2: Channel<'d, LowSpeed>,
    speed: f32,
}

impl<'d> Motor<'d> {
    pub fn new(channel1: Channel<'d, LowSpeed>, channel2: Channel<'d, LowSpeed>) -> Self {
        Motor {
            channel1,
            channel2,
            speed: 0.0,
        }
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) -> Result<(), Error> {
        self.speed = speed;

        let min = 0.3;
        let duty = ((speed.abs() * (1.0 - min) + min) * 100.0) as u8;

        if speed > 0.0 {
            self.channel1.set_duty(duty)?;
            self.channel2.set_duty(0)?;
        } else {
            self.channel1.set_duty(0)?;
            self.channel2.set_duty(duty)?;
        }
        Ok(())
    }
}
