pub mod schedule;
mod lights;
mod temperature;
mod ato;
pub mod doser;

use ::uom::temp::*;
use ::devices::Devices;
use ::devices::PiGpio;
use ::devices::GpioPin;
use ::devices::Depth;
use std::io as io;

use self::temperature::TemperatureController;
use self::lights::{LightController, FadeSpeed};
use self::ato::AtoController;
use self::schedule::{Schedule, ScheduleLeg};
use self::doser::{DoserController, Dose};

use carboxyl::Stream;

pub use self::ato::Calibration;

pub struct AquariumController {
    light_controller: LightController,
    temp_controller: TemperatureController,
    ato_controller: AtoController,
    doser_controller: DoserController,
    pump_pin: GpioPin,
    pump_off_timeout_s: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum Component {
    Temperature,
    Ato,
    Lighting
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub component: Component,
    pub message: String,
}

pub struct Status {
    pub heater_on: bool,
    pub ato_pump_on: bool,
    pub cooler_on: bool,
    pub alerts: Vec<Alert>,
}

const TICK_RESOLUTION_MS: u64 = 5000;
const PUMP_TIMEOUT_S: u64 = 20 * 60;

impl AquariumController {
    pub fn new(schedule: Schedule, temp_min: Temperature<F>, temp_max: Temperature<F>, depth_low: Depth, depth_high: Depth, depth_calibration: Calibration, temp_stream: Stream<Temperature<F>>, depth_stream: Stream<Depth>, pump_rate_ml_min: f32) -> AquariumController {
        let mut pi_gpio = PiGpio::new();
        let pin0 =  pi_gpio.take_pin(0, true).unwrap();
        let pin1 = pi_gpio.take_pin(1, true).unwrap();
        let pin2 =  pi_gpio.take_pin(2, true).unwrap();
        let mut pin3 = pi_gpio.take_pin(3, true).unwrap();
        let pin4 = pi_gpio.take_pin(4, false).unwrap();
        let pin5 = pi_gpio.take_pin(5, false).unwrap();
        pin3.turn_on().unwrap();
        AquariumController {
            light_controller: LightController::new(schedule, pin4),
            temp_controller: TemperatureController::new(temp_min, temp_max, pin0, pin2, temp_stream),
            ato_controller: AtoController::new(depth_low, depth_high, depth_calibration, pin1, depth_stream),
            doser_controller: DoserController::new(pin5, pump_rate_ml_min),
            pump_pin: pin3,
            pump_off_timeout_s: u64::max_value(),
        }
    }

    pub fn schedule_updated(&mut self, schedule: Schedule) {
        self.light_controller.schedule_updated(schedule)
    }

    pub fn set_temp_range(&mut self, min: Temperature<F>, max: Temperature<F>) {
        self.temp_controller.set_range(min, max);
    }

    pub fn set_depth_settings(&mut self, low: Depth, high: Depth, calibration: Calibration) {
        self.ato_controller.set_settings(low, high, calibration)
    }
     
    pub fn set_doser_settings(&mut self, pump_rate_ml_min: f32, schedule: Vec<Dose>) {
        self.doser_controller.set_settings(pump_rate_ml_min, schedule);
    }

    pub fn tick(&mut self, devices: &mut Devices, ticks: u64) -> Result<(), io::Error> {
        try!(self.light_controller.tick(devices, ticks));
        self.next_tick(ticks)
            .map_or(Ok(()), |tick| self.run(tick))
    }

    pub fn enable_pump(&mut self, enabled: bool, tick_ms: u64) -> io::Result<()> {
        if !enabled {
            self.pump_off_timeout_s = (tick_ms / 1000) + PUMP_TIMEOUT_S;
        } else {
            self.pump_off_timeout_s = u64::max_value();
        }
        self.pump_pin.set(enabled)
    }

    pub fn set_viewing_mode(&mut self, enabled: bool, tick: u64, leg: ScheduleLeg) { 
        if enabled {
            self.light_controller.live_mode(leg, FadeSpeed::Slow, 60 * 10 * 1000, tick)
        } else { 
            self.light_controller.disable_live_mode(tick)
        }
    }

    pub fn live_mode(&mut self, tick: u64, leg: ScheduleLeg) {
        self.light_controller.live_mode(leg, FadeSpeed::DurationMS(0), 30 * 1000, tick)
    }

    pub fn status(&mut self) -> Status {
        let temp_status = self.temp_controller.status();
        Status {
            heater_on: temp_status.heater,
            cooler_on: temp_status.cooler,
            ato_pump_on: self.ato_controller.status(),
            alerts: temp_status.alerts.into_iter().map(|a| Alert { component: Component::Temperature, message: a }).collect(),
        }
    }

    // Run loop for when a tick overflows
    fn run(&mut self, tick_s: u64) -> io::Result<()> {
        try!(self.temp_controller.tick(tick_s));
        try!(self.ato_controller.tick(tick_s, &mut self.pump_pin));
        try!(self.check_pump(tick_s));
        try!(self.doser_controller.tick(tick_s));
        Ok(())
    }

    fn check_pump(&mut self, tick_s: u64) -> io::Result<()> {
        if tick_s > self.pump_off_timeout_s {
            self.enable_pump(true, tick_s)
        } else {
            Ok(())
        }
    }

    // increments the tick counter and returns Some if it overflowed
    fn next_tick(&mut self, ticks: u64) -> Option<u64> {
        if ticks % TICK_RESOLUTION_MS == 0 {
            Some(ticks / 1000)
        } else {
            None
        }
    }

}

