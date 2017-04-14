use ::devices::AvrController;
use std::io as io;
use ::devices::GpioPin; 

pub struct AtoController {
    low_point: u8,
    high_point: u8,
    pin: GpioPin,
    on_tick_m: u64,
    calibration: Calibration,
}

pub struct Calibration {
    pub low: u8,
    pub high: u8,
    pub highInches: f32,
    pub tankSurfaceArea: u16,
}

impl AtoController {
    pub fn new(low_point: u8, high_point: u8, calibration: Calibration, pin: GpioPin) -> AtoController {
        AtoController {
            low_point: low_point,
            high_point: high_point,
            pin: pin,
            on_tick_m: 0,
            calibration: calibration,
        }
    }

    pub fn get_depth(&mut self, avr_controller: &mut AvrController) -> io::Result<u8> {
        avr_controller.get_depth()
    }

    pub fn tick(&mut self, avr_controller: &mut AvrController, tick_m: u64) -> io::Result<()> {
        let depth = try!(self.get_depth(avr_controller));

        if depth < self.low_point {
            println!("Water level too low, toggling on: {:?}", depth);
            self.on_tick_m = tick_m;
            self.pin.turn_on() 
        // TODO: make configureable based on GPH of pump and estimated high - low
        } else if depth > self.high_point { 
            println!("Water level too high, toggling off: {:?}", depth);
            self.pin.turn_off()
        } else if self.on_tick_m > 0 && tick_m - self.on_tick_m > 60 {
            println!("Timed out depth after 60 seconds, toggling off: {:?}", depth);
            self.on_tick_m = 0;
            self.pin.turn_off()
        } else {
            Ok(())
        }
    }
}

