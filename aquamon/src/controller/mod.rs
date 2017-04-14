pub mod schedule;
mod lights;
mod temperature;
mod ato;

use ::uom::temp::*;
use ::devices::AvrController;
use ::devices::PiGpio;
use i2cdev::linux::LinuxI2CError;
use std::io as io;
use chrono::prelude::*;

use self::schedule::{Schedule, ScheduleLeg};
use self::temperature::TemperatureController;
use self::ato::AtoController;

pub use self::ato::Calibration;

pub struct AquariumController {
    schedule: Schedule,
    temp_controller: TemperatureController,
    avr_controller: AvrController,
    ato_controller: AtoController,
    live_mode_ticks_remaining: u8,
    tick_ms: u16,
    tick_counter: u64,
}

impl AquariumController {
    pub fn new(schedule: Schedule, avr_controller: AvrController, tick_ms: u16, temp_setpoint: Temperature<F>, depth_low: u8, depth_high: u8, depth_calibration: Calibration) -> AquariumController {
        let mut pi_gpio = PiGpio::new();
        AquariumController {
            schedule: schedule,
            avr_controller: avr_controller,
            live_mode_ticks_remaining: 0,
            tick_ms: tick_ms, 
            tick_counter: 0,
            temp_controller: TemperatureController::new(temp_setpoint, pi_gpio.take_pin(0).unwrap()),
            ato_controller: AtoController::new(depth_low, depth_high, depth_calibration, pi_gpio.take_pin(1).unwrap()),
        }
    }

    pub fn schedule_updated(&mut self, schedule: Schedule) {
        println!("Schedule updated: {:?}", schedule);
        self.schedule = schedule;
    }

    pub fn get_temp(&mut self) -> io::Result<Temperature<C>> {
        self.avr_controller.get_temp()
    }

    pub fn set_temp_setpoint(&mut self, set_point: Temperature<F>) {
        self.temp_controller.set_setpoint(set_point);
    }

    pub fn get_depth(&mut self) -> io::Result<u8> {
        self.avr_controller.get_depth()
    }

    pub fn tick(&mut self) -> Result<(), io::Error> {
        self.next_tick()
            .map_or(Ok(()), |tick| self.run(tick))
    }

    // Run loop for when a tick overflows
    fn run(&mut self, tick_m: u64) -> io::Result<()> {
        let local_time = UTC::now().with_timezone(&Local);
        // shift the current time because the scheule legs use a NaiveTime
        let time = local_time.time();
        let intensities = self.schedule.get_intensities(time);
        println!("setting intensities: {:?}", intensities);

        self.avr_controller.set_intensities(&intensities)
            .map_err(From::from)
            .and_then(|_| self.temp_controller.tick(&mut self.avr_controller))
            .and_then(|_| self.ato_controller.tick(&mut self.avr_controller, tick_m))
    }

    // increments the tick counter and returns Some if it overflowed
    fn next_tick(&mut self) -> Option<u64> {
        self.tick_counter += 1;

        if self.tick_counter % (60000 / self.tick_ms as u64) == 0 {
            if self.live_mode_ticks_remaining > 0 { 
                self.live_mode_ticks_remaining -= 1; 
                None
            } else {
                Some(self.tick_counter / (60000 / self.tick_ms as u64))
            }
        } else {
            None
        }
    }

    pub fn live_mode(&mut self, leg: ScheduleLeg, live_mode_ticks: u8) -> Result<(), LinuxI2CError> {
        let intensities = leg.weighted_intensities();
        println!("Intensities: {:?}", intensities);
        self.live_mode_ticks_remaining = live_mode_ticks;
        self.avr_controller.set_intensities(&intensities)
    }
}

