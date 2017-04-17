use ::uom::temp::*;
use std::io as io;
use ::devices::GpioPin; 
use carboxyl::{Stream, Signal};

pub struct TemperatureController {
    set_point: Temperature<F>,
    hysteresis: Temperature<F>,
    pin: GpioPin,
    on_tick_m: u64,
    timed_out_tick_m: u64,
    temp_sink: Signal<Temperature<F>>,
}

const HEATER_MAX_CYCLE_TIME: u64 = 5;

impl TemperatureController {
    pub fn new(set_point: Temperature<F>, pin: GpioPin, temp_stream: Stream<Temperature<F>>) -> TemperatureController {
        let temp = Temperature::in_f(80.0);
        TemperatureController {
            set_point: set_point,
            hysteresis: Temperature::in_f(0.3),
            pin: pin,
            on_tick_m: 0,
            timed_out_tick_m: 0,
            temp_sink: temp_stream.fold((temp, temp, temp, temp), |(b, c, d, _), a| (a, b, c, d))
                                  .map(|(a,b,c,d)| Temperature::in_f((a + b + c + d).value() / 4.0))
        }
    }

    pub fn set_setpoint(&mut self, set_point: Temperature<F>) {
        self.set_point = set_point;
    }

    pub fn tick(&mut self, tick_m: u64) -> io::Result<()> {
        if self.timed_out_tick_m > 0 && self.timed_out_tick_m + 5 > tick_m {
            return Ok(());
        }
        // TODO: PID
        let temp = self.temp_sink.sample();
         
        if  temp > self.set_point + self.hysteresis {
            info!("Toggling temp off: {:?}", temp.value());
            self.on_tick_m = 0;
            self.pin.turn_off()
        } else if temp < self.set_point - self.hysteresis && self.on_tick_m == 0 {
            info!("Toggling temp on: {:?}", temp.value());
            self.on_tick_m = tick_m;
            self.pin.turn_on()
        } else if self.on_tick_m > 0 && tick_m - self.on_tick_m > HEATER_MAX_CYCLE_TIME {
            info!("Heater timed out after {:?} minutes", HEATER_MAX_CYCLE_TIME);
            self.on_tick_m = 0;
            self.timed_out_tick_m = tick_m;
            self.pin.turn_off()
        } else {
            Ok(())
        }
    }
}
