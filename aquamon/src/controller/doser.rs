use std::io as io;
use ::devices::GpioPin;
use chrono::prelude::*;
use chrono::Duration;

pub struct DoserController {
    pin: GpioPin,
    pump_rate_ml_min: f32,
    schedule: Vec<Dose>,
}

#[derive(Debug)]
pub struct Dose {
    pub dose_amount_ml: f32,
    pub start_time: NaiveTime,
}

const TICK_RESOLUTION_MS: u64 = 1000;

impl DoserController {
    pub fn new(pin: GpioPin, pump_rate_ml_min: f32) -> DoserController {
        DoserController {
            pin: pin,
            pump_rate_ml_min: pump_rate_ml_min,
            schedule: vec![],
        }
    }

    pub fn set_settings(&mut self, pump_rate_ml_min: f32, schedule: Vec<Dose>) {
        info!("Doser settings updated. Pump rate: {:?}, schedule: {:?}", pump_rate_ml_min, schedule);
        self.schedule = schedule;
        self.pump_rate_ml_min = pump_rate_ml_min;
    }

    pub fn tick(&mut self, tick_ms: u64) -> io::Result<()> {
        if tick_ms % TICK_RESOLUTION_MS != 0 { return Ok(()); }

        let local_time = UTC::now().with_timezone(&Local);
        let time = local_time.time();

        let cur_dose = self.schedule
            .iter()
            .find(|dose| dose.start_time >= time && dose.get_end_time(self.pump_rate_ml_min) <= time);

        if cur_dose.is_some() {
            info!("Doser: dosing {:?}mL", cur_dose.unwrap().dose_amount_ml); 
            self.pin.turn_on()
        } else {
            self.pin.turn_off()
        }
    }
}

impl Dose {
    fn get_end_time(&self, pump_rate_ml_min: f32) -> NaiveTime {
        let ml_s = pump_rate_ml_min / 60.0;
        let seconds = self.dose_amount_ml / ml_s;
        self.start_time + Duration::seconds(seconds.round() as i64)
    }
}

