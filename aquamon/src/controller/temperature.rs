use ::uom::temp::*;
use std::io as io;
use ::devices::GpioPin; 
use carboxyl::{Stream, Signal};

pub struct TemperatureController {
    min: Temperature<F>,
    max: Temperature<F>,
    hysteresis: Temperature<F>,
    pin: GpioPin,
    cool_pin: GpioPin,
    on_tick_s: u64,
    cool_on_tick_s: u64,
    timed_out_tick_s: u64,
    temp_sink: Signal<Temperature<F>>,
}

pub struct Status {
    pub heater: bool, 
    pub cooler: bool,
    pub alerts: Vec<String>,
}

const HEATER_MAX_CYCLE_TIME: u64 = 12 * 60 * 60; // 12 hours * 60 minutes * 60 seconds
const TIMEOUT_PERIOD: u64 = 10 * 60;

impl TemperatureController {
    pub fn new(min: Temperature<F>, max: Temperature<F>, pin: GpioPin, cool_pin: GpioPin, temp_stream: Stream<Temperature<F>>) -> TemperatureController {
        let temp = Temperature::in_f(80.0);
        TemperatureController {
            min: min,
            max: max,
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

    pub fn set_range(&mut self, min: Temperature<F>, max: Temperature<F>) {
        self.min = min;
        self.max = max;
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
         
        if temp > self.min + self.hysteresis && self.on_tick_s != 0 {
            info!("Toggling temp off: {:?}", temp.value());
            self.on_tick_s = 0;
            self.pin.turn_off()
        } else if temp > self.max + self.hysteresis && self.on_tick_s == 0 && self.cool_on_tick_s == 0 {
            info!("Toggling cooling on: {:?}", temp.value());
            self.cool_on_tick_s = tick_s;
            self.cool_pin.turn_on()
        } else if temp < self.max - self.hysteresis && self.cool_on_tick_s != 0 {
            info!("Toggling cooling off: {:?}", temp.value());
            self.cool_on_tick_s = 0;
            self.cool_pin.turn_off()
        } else if temp < self.min - self.hysteresis && self.on_tick_s == 0 {
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
}
