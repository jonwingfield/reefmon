use ::devices::AvrController;
use ::uom::temp::*;
use std::io as io;
use ::devices::GpioPin; 

pub struct TemperatureController {
    set_point: Temperature<F>,
    hysteresis: Temperature<F>,
    pin: GpioPin,
}

impl TemperatureController {
    pub fn new(set_point: Temperature<F>, pin: GpioPin) -> TemperatureController {
        TemperatureController {
            set_point: set_point,
            hysteresis: Temperature::in_f(0.25),
            pin: pin
        }
    }

    pub fn get_temp(&mut self, avr_controller: &mut AvrController) -> io::Result<Temperature<C>> {
        avr_controller.get_temp()
    }

    pub fn set_setpoint(&mut self, set_point: Temperature<F>) {
        self.set_point = set_point;
    }

    pub fn tick(&mut self, avr_controller: &mut AvrController) -> io::Result<()> {
        // TODO: PID
        // TODO: limit duration and space between tries
        let temp = try!(self.get_temp(avr_controller)).to_f();
         
        if  temp > self.set_point + self.hysteresis {
            println!("Toggling temp off: {:?}", temp.value());
            self.pin.turn_off()
        } else if temp < self.set_point - self.hysteresis {
            println!("Toggling temp on: {:?}", temp.value());
            self.pin.turn_on()
        } else {
            Ok(())
        }
    }
}
