use std::io as io;
use ::devices::GpioPin;
use chrono::prelude::*;
use chrono::Duration;

pub struct DoserController {
    pin: GpioPin,
    stream: DoserStream,
}

struct DoserStream {
    pump_rate_ml_min: f32,
    schedule: Vec<Dose>,
}

#[derive(Debug)]
pub struct Dose {
    pub dose_amount_ml: f32,
    pub start_time: NaiveTime,
}

impl DoserController {
    pub fn new(pin: GpioPin, pump_rate_ml_min: f32) -> DoserController {
        DoserController {
            pin: pin,
            stream: DoserStream {
                pump_rate_ml_min: pump_rate_ml_min,
                schedule: vec![],
            }
        }
    }

    pub fn set_settings(&mut self, pump_rate_ml_min: f32, schedule: Vec<Dose>) {
        info!("Doser settings updated. Pump rate: {:?}, schedule: {:?}", pump_rate_ml_min, schedule);
        self.stream.schedule = schedule;
        self.stream.pump_rate_ml_min = pump_rate_ml_min;
    }

    pub fn tick(&mut self, tick_ms: u64) -> io::Result<()> {
        // only check every second
        if tick_ms % 1000 != 0 { return Ok(()); }

        let time = UTC::now().with_timezone(&Local).time();

        if let Some(dose) = self.stream.tick(time) {
            info!("Doser: dosing {:?}mL", dose.dose_amount_ml); 
            self.pin.turn_on()
        } else {
            self.pin.turn_off()
        }
    }
}

impl DoserStream {
    pub fn tick<'a>(&'a self, time: NaiveTime) -> Option<&'a Dose> {
        self.schedule
            .iter()
            .find(|dose| dose.start_time <= time && dose.get_end_time(self.pump_rate_ml_min) >= time)
    }
}

impl Dose {
    fn get_end_time(&self, pump_rate_ml_min: f32) -> NaiveTime {
        let ml_s = pump_rate_ml_min / 60.0;
        let seconds = self.dose_amount_ml / ml_s;
        self.start_time + Duration::seconds(seconds.round() as i64)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn doser_start_end_time() {
        let dose = Dose {
            dose_amount_ml: 0.4,
            start_time: NaiveTime::from_hms(7, 0, 0),
        };

        assert_eq!(dose.get_end_time(1.1), NaiveTime::from_hms(7, 0, 22));
    }

    #[test]
    fn doser_on() {
        let dose = Dose {
            dose_amount_ml: 0.4,
            start_time: NaiveTime::from_hms(7, 0, 0),
        };

        let stream = DoserStream {
            pump_rate_ml_min: 1.1,
            schedule: vec![dose]
        };

        assert!(stream.tick(NaiveTime::from_hms(7, 0, 0)).is_some());
        assert!(stream.tick(NaiveTime::from_hms(7, 0, 22)).is_some());
        assert!(stream.tick(NaiveTime::from_hms(7, 0, 15)).is_some());
    }

    #[test]
    fn doser_off() {
        let dose = Dose {
            dose_amount_ml: 0.4,
            start_time: NaiveTime::from_hms(7, 0, 0),
        };

        let stream = DoserStream {
            pump_rate_ml_min: 1.1,
            schedule: vec![dose]
        };

        assert!(stream.tick(NaiveTime::from_hms(6, 59, 59)).is_none());
        assert!(stream.tick(NaiveTime::from_hms(7, 0, 23)).is_none());
        assert!(stream.tick(NaiveTime::from_hms(9, 0, 23)).is_none());
    }
}
