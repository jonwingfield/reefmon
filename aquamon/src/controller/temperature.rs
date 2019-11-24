use ::uom::temp::*;
use std::io as io;
use ::devices::GpioPin; 
use carboxyl::{Stream, Signal};
use chrono::prelude::*;

pub struct TemperatureController {
    cooler_range: TemperatureRange,
    heater_range: TemperatureRange,
    hysteresis: Temperature<F>,
    pin: GpioPin,
    cool_pin: GpioPin,
    on_tick_s: u64,
    cool_on_tick_s: u64,
    timed_out_tick_s: u64,
    temp_sink: Signal<Temperature<F>>,
}

pub struct TemperatureRange {
    pub min: Temperature<F>,
    pub min_time: NaiveTime,
    pub max: Temperature<F>,
    pub max_time: NaiveTime,
}

const SECONDS_IN_DAY: u32 = 60 * 60 * 24;

fn sub_seconds_absolute(first: u32, second: u32) -> u32 {
    if second > first {
        SECONDS_IN_DAY - second + first
    } else {
        first - second
    }
}

impl TemperatureRange {
    pub fn discrete(temp: Temperature<F>) -> TemperatureRange {
        TemperatureRange {
            min: temp,
            max: temp,
            min_time: NaiveTime::from_hms(0,0,0),
            max_time: NaiveTime::from_hms(0,0,0),
        }
    }

    fn temperature_for_time(&self, time: NaiveTime) -> Temperature<F> {
        let min_s = self.min_time.num_seconds_from_midnight();
        let max_s = self.max_time.num_seconds_from_midnight();
        let now_s = time.num_seconds_from_midnight();

        if max_s < min_s || max_s - min_s == 0 {
            return self.min;
        }

        let diff_temp = self.max - self.min;

        if now_s < min_s || now_s > max_s {
            let percent_elapsed = sub_seconds_absolute(now_s, max_s) as f32 / sub_seconds_absolute(min_s, max_s) as f32;
            let interpolated = percent_elapsed * diff_temp.value();
            Temperature::in_f(self.max.value() - (interpolated * 10.0).round() / 10.0)
        } else {
            let percent_elapsed = (now_s- min_s) as f32 / (max_s - min_s) as f32;
            let interpolated = percent_elapsed * diff_temp.value();
            Temperature::in_f(self.min.value() + (interpolated * 10.0).round() / 10.0)
        }
    }
}

pub struct Status {
    pub heater: bool, 
    pub cooler: bool,
    pub alerts: Vec<String>,
}

const HEATER_MAX_CYCLE_TIME: u64 = 24 * 60 * 60; // 12 hours * 60 minutes * 60 seconds
const TIMEOUT_PERIOD: u64 = 10 * 60;

impl TemperatureController {
    pub fn new(heater_range: TemperatureRange, cooler_range: TemperatureRange, pin: GpioPin, cool_pin: GpioPin, temp_stream: Stream<Temperature<F>>) -> TemperatureController {
        let temp = Temperature::in_f(80.0);
        TemperatureController {
            heater_range: heater_range,
            cooler_range: cooler_range,
            hysteresis: Temperature::in_f(0.2),
            pin: pin,
            cool_pin: cool_pin,
            on_tick_s: 0,
            cool_on_tick_s: 0,
            timed_out_tick_s: 0,
            temp_sink: temp_stream.fold((temp, temp, temp, temp), |(b, c, d, _), a| (a, b, c, d))
                                  .map(|(a,b,c,d)| Temperature::in_f((a + b + c + d).value() / 4.0))
        }
    }

    pub fn set_range(&mut self, heater_range: TemperatureRange, cooler_range: TemperatureRange) {
        self.heater_range = heater_range;
        self.cooler_range = cooler_range;
    }

    pub fn status(&mut self) -> Status { 
        let temp = self.temp_sink.sample();
        Status {
            heater: self.on_tick_s > 0, 
            cooler: self.cool_on_tick_s > 0,
            alerts: if temp >= Temperature::in_f(84.0) { vec![format!("Temperature too high: {}", temp)] } else { vec![] }
        }
    }

    pub fn tick(&mut self, tick_s: u64) -> io::Result<()> {
        if self.timed_out_tick_s > 0 && self.timed_out_tick_s + TIMEOUT_PERIOD > tick_s {
            return Ok(());
        }
        let temp = self.temp_sink.sample();
        let local_time = UTC::now().with_timezone(&Local).time();
        let min = self.get_min(local_time);
        let max = self.get_max(local_time);
        info!("Calculated min: {:?} max: {:?}", min, max);
         
        if temp > min + self.hysteresis && self.on_tick_s != 0 {
            info!("Toggling temp off: {:?}", temp.value());
            self.on_tick_s = 0;
            self.pin.turn_off()
        } else if temp > max + self.hysteresis && self.on_tick_s == 0 && self.cool_on_tick_s == 0 {
            info!("Toggling cooling on: {:?}", temp.value());
            self.cool_on_tick_s = tick_s;
            self.cool_pin.turn_on()
        } else if temp < max - self.hysteresis && self.cool_on_tick_s != 0 {
            info!("Toggling cooling off: {:?}", temp.value());
            self.cool_on_tick_s = 0;
            self.cool_pin.turn_off()
        } else if temp < min - self.hysteresis && self.on_tick_s == 0 {
            info!("Toggling temp on: {:?}", temp.value());
            self.on_tick_s = tick_s;
            self.pin.turn_on()
        } else if self.on_tick_s > 0 && tick_s - self.on_tick_s > HEATER_MAX_CYCLE_TIME {
            info!("Heater timed out after {:?} minutes", HEATER_MAX_CYCLE_TIME);
            self.on_tick_s = 0;
            self.timed_out_tick_s = tick_s;
            self.pin.turn_off()
        } else {
            Ok(())
        }
    }

    fn get_min(&self, now: NaiveTime) -> Temperature<F> {
        self.heater_range.temperature_for_time(now)
    }

    fn get_max(&self, now: NaiveTime) -> Temperature<F> {
        self.cooler_range.temperature_for_time(now)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn temperature_between_times() {
        let range = TemperatureRange {
            min: Temperature::in_f(77.0),
            min_time: NaiveTime::from_hms(10, 0, 0),
            max: Temperature::in_f(79.0),
            max_time: NaiveTime::from_hms(15, 0, 0),
        };

        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(12, 40, 0)),
            Temperature::in_f(78.1));
    }

    #[test]
    fn temperature_before_times() {
        let range = TemperatureRange {
            min: Temperature::in_f(77.0),
            min_time: NaiveTime::from_hms(10, 0, 0),
            max: Temperature::in_f(79.0),
            max_time: NaiveTime::from_hms(15, 0, 0),
        };

        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(9, 15, 0)),
            Temperature::in_f(77.1));
    }

    #[test]
    fn temperature_after_times() {
        let range = TemperatureRange {
            min: Temperature::in_f(77.0),
            min_time: NaiveTime::from_hms(10, 0, 0),
            max: Temperature::in_f(79.0),
            max_time: NaiveTime::from_hms(15, 0, 0),
        };

        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(19, 15, 0)),
            Temperature::in_f(78.6));
    }

    #[test]
    fn temperature_at_time() {
        let range = TemperatureRange {
            min: Temperature::in_f(77.0),
            min_time: NaiveTime::from_hms(10, 0, 0),
            max: Temperature::in_f(79.0),
            max_time: NaiveTime::from_hms(15, 0, 0),
        };

        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(10, 0, 0)),
            Temperature::in_f(77.0));
        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(15, 0, 0)),
            Temperature::in_f(79.0));

    }

    #[test]
    fn zero_times() {
        let range = TemperatureRange {
            min: Temperature::in_f(77.0),
            min_time: NaiveTime::from_hms(0, 0, 0),
            max: Temperature::in_f(79.0),
            max_time: NaiveTime::from_hms(0, 0, 0),
        };

        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(19, 15, 0)),
            Temperature::in_f(77.0));
    }

    #[test]
    fn zero_min() {
        let range = TemperatureRange {
            min: Temperature::in_f(77.0),
            min_time: NaiveTime::from_hms(0, 0, 0),
            max: Temperature::in_f(79.0),
            max_time: NaiveTime::from_hms(12, 0, 0),
        };

        assert_eq!(
            range.temperature_for_time(NaiveTime::from_hms(6, 30, 0)),
            Temperature::in_f(78.1));
    }
}
