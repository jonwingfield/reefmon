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

impl AquariumController {
    pub fn new(schedule: Schedule, temp_setpoint: Temperature<F>, depth_low: u8, depth_high: u8, depth_calibration: Calibration, temp_stream: Stream<Temperature<F>>, depth_stream: Stream<Depth>) -> AquariumController {
        let mut pi_gpio = PiGpio::new();
        AquariumController {
            schedule: schedule,
            live_mode_ticks_remaining: 0,
            temp_controller: TemperatureController::new(temp_setpoint, pi_gpio.take_pin(0, true).unwrap(), temp_stream),
            ato_controller: AtoController::new(depth_low, depth_high, depth_calibration, pi_gpio.take_pin(1, true).unwrap(), depth_stream),
        }
    }

    pub fn schedule_updated(&mut self, schedule: Schedule) {
        info!("Schedule updated: {:?}", schedule);
        self.schedule = schedule;
    }

    pub fn set_temp_setpoint(&mut self, set_point: Temperature<F>) {
        self.temp_controller.set_setpoint(set_point);
    }

    pub fn set_depth_settings(&mut self, low: u8, high: u8, calibration: Calibration) {
        self.ato_controller.set_settings(low, high, calibration)
    }

    pub fn tick(&mut self, devices: &mut Devices, ticks: u64) -> Result<(), io::Error> {
        self.next_tick(ticks)
            .map_or(Ok(()), |tick| self.run(devices, tick))
    }

    // Run loop for when a tick overflows
    fn run(&mut self, devices: &mut Devices, tick_m: u64) -> io::Result<()> {
        let local_time = UTC::now().with_timezone(&Local);
        // shift the current time because the scheule legs use a NaiveTime
        let time = local_time.time();
        let intensities = self.schedule.get_intensities(time);
        info!("setting intensities: {:?}", intensities);

        devices.set_intensities(&intensities)
            .map_err(From::from)
            .and_then(|_| self.temp_controller.tick(tick_m))
            .and_then(|_| self.ato_controller.tick(tick_m))
    }

    // increments the tick counter and returns Some if it overflowed
    fn next_tick(&mut self, ticks: u64) -> Option<u64> {
        if ticks % 60000 == 0 {
            if self.live_mode_ticks_remaining > 0 { 
                self.live_mode_ticks_remaining -= 1; 
                None
            } else {
                Some(ticks / 60000)
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

