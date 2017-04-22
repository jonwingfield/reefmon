pub mod schedule;
mod lights;
mod temperature;
mod ato;

use ::uom::temp::*;
use ::devices::Devices;
use ::devices::PiGpio;
use ::devices::Depth;
use std::io as io;
use chrono::prelude::*;

use self::schedule::{Schedule, ScheduleLeg};
use self::temperature::TemperatureController;
use self::ato::AtoController;

use carboxyl::Stream;

pub use self::ato::Calibration;

pub struct AquariumController {
    schedule: Schedule,
    temp_controller: TemperatureController,
    ato_controller: AtoController,
    live_mode_ticks_remaining: u8,
}

pub struct Status {
    pub heater_on: bool,
    pub ato_pump_on: bool,
    pub cooler_on: bool,
}

impl AquariumController {
    pub fn new(schedule: Schedule, temp_min: Temperature<F>, temp_max: Temperature<F>, depth_low: Depth, depth_high: Depth, depth_calibration: Calibration, temp_stream: Stream<Temperature<F>>, depth_stream: Stream<Depth>) -> AquariumController {
        let mut pi_gpio = PiGpio::new();
        let pin0 =  pi_gpio.take_pin(0, true).unwrap();
        let pin1 = pi_gpio.take_pin(1, true).unwrap();
        let pin2 =  pi_gpio.take_pin(2, true).unwrap();
        let mut pin3 = pi_gpio.take_pin(3, true).unwrap();
        pin3.turn_on().unwrap();
        AquariumController {
            schedule: schedule,
            live_mode_ticks_remaining: 0,
            temp_controller: TemperatureController::new(temp_min, temp_max, pin0, pin2, temp_stream),
            ato_controller: AtoController::new(depth_low, depth_high, depth_calibration, pin1, depth_stream),
        }
    }

    pub fn schedule_updated(&mut self, schedule: Schedule) {
        info!("Schedule updated: {:?}", schedule);
        self.schedule = schedule;
    }

    pub fn set_temp_range(&mut self, min: Temperature<F>, max: Temperature<F>) {
        self.temp_controller.set_range(min, max);
    }

    pub fn set_depth_settings(&mut self, low: Depth, high: Depth, calibration: Calibration) {
        self.ato_controller.set_settings(low, high, calibration)
    }

    pub fn tick(&mut self, devices: &mut Devices, ticks: u64) -> Result<(), io::Error> {
        self.next_tick(ticks)
            .map_or(Ok(()), |tick| self.run(devices, tick))
    }

    pub fn status(&mut self) -> Status {
        let temp_status = self.temp_controller.status();
        Status {
            heater_on: temp_status.0,
            cooler_on: temp_status.1,
            ato_pump_on: self.ato_controller.status(),
        }
    }

    // Run loop for when a tick overflows
    fn run(&mut self, devices: &mut Devices, tick_s: u64) -> io::Result<()> {
        let local_time = UTC::now().with_timezone(&Local);
        // shift the current time because the scheule legs use a NaiveTime
        let time = local_time.time();
        let intensities = self.schedule.get_intensities(time);
        info!("setting intensities: {:?}", intensities);

        devices.set_intensities(&intensities)
            .map_err(From::from)
            .and_then(|_| self.temp_controller.tick(tick_s))
            .and_then(|_| self.ato_controller.tick(tick_s))
    }

    // increments the tick counter and returns Some if it overflowed
    fn next_tick(&mut self, ticks: u64) -> Option<u64> {
        if ticks % 30000 == 0 {
            if self.live_mode_ticks_remaining > 0 { 
                self.live_mode_ticks_remaining -= 1; 
                None
            } else {
                Some(ticks / 1000)
            }
        } else {
            None
        }
    }

    pub fn live_mode(&mut self, devices: &mut Devices, leg: ScheduleLeg, live_mode_ticks: u8) -> io::Result<()> {
        let intensities = leg.weighted_intensities();
        info!("Intensities: {:?}", intensities);
        self.live_mode_ticks_remaining = live_mode_ticks;
        devices.set_intensities(&intensities)
    }
}

