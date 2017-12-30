use super::schedule::{Schedule, ScheduleLeg, Intensities, interpolated_intensity};

use std::io as io;
use std::cmp;

use ::devices::GpioPin;

use chrono::prelude::*;
 
use ::devices::Devices;

struct LiveMode {
    pub end_tick: u64,
    pub intensities: (Intensities, Intensities),
    pub fade_duration_ticks: u64,
    pub start_tick: u64,
}
 
impl LiveMode {
    pub fn new( end_tick: u64, intensities: (Intensities, Intensities), fade_duration_ticks: u64, start_tick: u64) -> LiveMode {
        LiveMode { end_tick: end_tick, intensities: intensities, fade_duration_ticks: fade_duration_ticks, start_tick: start_tick }
    }
}

pub struct LightController {
    schedule: Schedule,
    live_mode: LiveMode,
    fuge_light_pin: GpioPin,
}

pub enum FadeSpeed {
    Slow, 
    #[allow(dead_code)]
    Fast, 
    DurationMS(u64),
}

impl LightController {
    pub fn new(schedule: Schedule, fuge_light_pin: GpioPin) -> LightController {
        LightController {
            schedule: schedule,
            live_mode: LiveMode::new(0, ([0_u8; 6], [0_u8; 6]), 0, 0),
            fuge_light_pin: fuge_light_pin,
        }
    }

    pub fn schedule_updated(&mut self, schedule: Schedule) {
        info!("Schedule updated: {:?}", schedule);
        self.schedule = schedule;
    }

    pub fn tick(&mut self, devices: &mut Devices, tick: u64) -> io::Result<()> {
        if self.live_mode.end_tick < tick && tick % 30 != 0 {
            return Ok(());
        }
        let local_time = UTC::now().with_timezone(&Local);
        // shift the current time because the scheule legs use a NaiveTime
        let time = local_time.time();
         

        let intensities = if self.live_mode.end_tick < tick {
            self.schedule.get_intensities(time)
        } else if self.live_mode.fade_duration_ticks == 0 {
            self.live_mode.intensities.1
        } else {
            let percent = (tick - self.live_mode.start_tick) as f32 / self.live_mode.fade_duration_ticks as f32;
            interpolated_intensity(&self.live_mode.intensities.0, 
                                   &self.live_mode.intensities.1,
                                   percent)
        };

        let fuge_light_on = time > NaiveTime::from_hms(17,30,0) || time < NaiveTime::from_hms(5,30,0);

        self.fuge_light_pin.set(fuge_light_on)
            .and_then(|_| devices.set_intensities(&intensities))
            .map_err(From::from)
    }

    pub fn live_mode(&mut self, leg: ScheduleLeg, fade_speed: FadeSpeed, duration_ticks: u64, tick: u64){
        let local_time = UTC::now().with_timezone(&Local);
        // shift the current time because the scheule legs use a NaiveTime
        let time = local_time.time();
        let current_intensities = self.schedule.get_intensities(time);

        let fade_duration_ticks = match fade_speed {
            FadeSpeed::Slow => 20000,
            FadeSpeed::Fast => 2000,
            FadeSpeed::DurationMS(ms) => ms
        };
        let live_mode_end_tick = cmp::max(duration_ticks + tick, self.live_mode.end_tick);
        self.live_mode = LiveMode::new(live_mode_end_tick, (current_intensities, leg.weighted_intensities()), fade_duration_ticks, tick);
    }

    pub fn disable_live_mode(&mut self, tick: u64) {
        let local_time = UTC::now().with_timezone(&Local);
        // shift the current time because the scheule legs use a NaiveTime
        let time = local_time.time();
         
        let current_intensities = self.schedule.get_intensities(time);

        self.live_mode = LiveMode::new(tick + 20000, (self.live_mode.intensities.1, current_intensities), 20000, tick);
    }
}
