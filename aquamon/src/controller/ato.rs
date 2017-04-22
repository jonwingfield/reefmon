use std::io as io;
use ::devices::GpioPin; 
use ::devices::Depth;
use carboxyl::{Signal,Stream};

pub struct AtoController {
    low_point: Depth,
    high_point: Depth,
    pin: GpioPin,
    on_tick_s: u64,
    timed_out_tick_s: u64,
    calibration: Calibration,
    depth_signal: Signal<Depth>,
}

pub struct Calibration {
    pub low: Depth,
    pub high: Depth,
    pub high_inches: f32,
    pub tank_surface_area: u16,
    pub pump_gph: f32,
}

const CU_IN_PER_GALLON: f32 = 231.0;
const TIMEOUT_PERIOD: u64 = 60 * 60;
 
impl Calibration {
    fn run_time_s(&self, low_point: Depth, high_point: Depth) -> u64 {
        // 10.0
        let steps_per_in: f32 = (self.high as f32 - self.low as f32) / self.high_inches; 

        // 0.0016666
        let pump_gps = self.pump_gph / 60.0 / 60.0;
        // 100/231 = 0.432
        let gallons_per_in = (self.tank_surface_area as f32) / CU_IN_PER_GALLON;

        // 100 - 70 / 10 = 3.0
        let in_to_rise = (high_point as f32 - low_point as f32) / steps_per_in;
        // 3.0 * 0.432 = 1.296
        let gallons_to_pump = in_to_rise * gallons_per_in;

        (gallons_to_pump / pump_gps) as u64
    }
}

impl AtoController {
    pub fn new(low_point: Depth, high_point: Depth, calibration: Calibration, pin: GpioPin, depth_stream: Stream<Depth>) -> AtoController {
        let i = 400;
        AtoController {
            low_point: low_point,
            high_point: high_point,
            pin: pin,
            on_tick_s: 0,
            timed_out_tick_s: 0,
            calibration: calibration,
            depth_signal: depth_stream.hold(i),
                // .fold((i,i,i,i,i,i,i,i,i,i), |(b,c,d,e,f,g,h,i,j,_), a| (a,b,c,d,e,f,g,h,i,j))
                // .map(|(a,b,c,d,e,f,g,h,i,j)| (a+b+c+d+e+f+g+h+i+j) / 10),
        }
    }

    pub fn set_settings(&mut self, low: Depth, high: Depth, calibration: Calibration) {
        self.low_point = low;
        self.high_point = high;
        self.calibration = calibration;
    }

    pub fn status(&mut self) -> bool { self.on_tick_s > 0 }

    pub fn tick(&mut self,  tick_s: u64) -> io::Result<()> {
        if self.timed_out_tick_s > 0 && self.timed_out_tick_s + TIMEOUT_PERIOD > tick_s {
            return Ok(());
        }
        let depth = self.depth_signal.sample();
        info!("Depth settings: {:?}-{:?}. Current: {:?}", self.low_point, self.high_point, depth);

        if depth < self.low_point && self.on_tick_s == 0 {
            if depth < self.low_point - 20 {
                error!("Water level too far below bottom point, please manually fill the tank. This is to avoid overflows when the sensor isn't attached properly, during water changes, etc.");
                return Ok(());
            }
            info!("Water level too low, toggling on: {:?}", depth);
            self.on_tick_s = tick_s;
            self.pin.turn_on() 
        // TODO: make configureable based on GPH of pump and estimated high - low
        } else if depth >= self.high_point { 
            info!("Water level too high, toggling off: {:?}", depth);
            self.on_tick_s = 0;
            self.pin.turn_off()
        } else if self.on_tick_s > 0 && tick_s - self.on_tick_s > self.calibration.run_time_s(self.low_point, self.high_point)  { 
            // TODO: alert should happen if the pump times out
            info!("Timed out depth after calculated seconds ({:?}), toggling off: {:?}", 
                     self.calibration.run_time_s(self.low_point, self.high_point), depth);
            self.on_tick_s = 0;
            self.timed_out_tick_s = tick_s;
            self.pin.turn_off()
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn run_time() {
        let calibration = Calibration {
            low: 60, 
            high: 120,
            high_inches: 6.0, // 10 steps per inch
            tank_surface_area: 100,
            pump_gph: 5.0 // .2 gpm 
        };

        // raise 3 inches (30 steps / 10 steps per inch)
        assert_eq!(calibration.run_time_s(70, 100), 935);
    }

}
