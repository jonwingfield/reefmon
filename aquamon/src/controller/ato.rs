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
    prev_high: (u64, Depth),
    recent_high: Signal<Depth>,
    failed_safe: bool,
    pump_off: u64,
}

pub struct Calibration {
    pub low: Depth,
    pub high: Depth,
    pub high_inches: f32,
    pub tank_surface_area: u16,
    pub pump_gph: f32,
    pub tank_volume: f32,
}

const CU_IN_PER_GALLON: f32 = 231.0;
const TIMEOUT_PERIOD: u64 = 2 * 60 * 60;
const MAX_EVAP_GPH: f32 = 1.0 / 8.0;
 
impl Calibration {
    fn run_time_s(&self, low_point: Depth, high_point: Depth) -> u64 {
        let steps_per_in: f32 = (self.high as f32 - self.low as f32) / self.high_inches; 

        let pump_gps = self.pump_gph / 60.0 / 60.0;
        let gallons_per_in = (self.tank_surface_area as f32) / CU_IN_PER_GALLON;

        let in_to_rise = (high_point as f32 - low_point as f32) / steps_per_in;
        let gallons_to_pump = in_to_rise * gallons_per_in;

        (gallons_to_pump / pump_gps) as u64
    }

    fn max_evap_per_hour(&self) -> Depth {
        let steps_per_in: f32 = (self.high as f32 - self.low as f32) / self.high_inches; 

        let gal_per_in = self.tank_surface_area as f32 / CU_IN_PER_GALLON;

        (steps_per_in / gal_per_in * MAX_EVAP_GPH) as Depth
    }
}

impl AtoController {
    pub fn new(low_point: Depth, high_point: Depth, calibration: Calibration, pin: GpioPin, depth_stream: Stream<Depth>) -> AtoController {
        let i = 400;
        let recent_high = depth_stream.fold(([0_u16; 32], 0), |(mut hist, index), next| {
            hist[index/6] = next;
            let next = if (index + 1) / 6 >= hist.len() { 0 } else { index + 1 };
            (hist, next)
        }).map(|(hist, _)| *hist.iter().max().unwrap());
        AtoController {
            low_point: low_point,
            high_point: high_point,
            pin: pin,
            on_tick_s: 0,
            timed_out_tick_s: 0,
            calibration: calibration,
            depth_signal: depth_stream.hold(i),
            recent_high: recent_high,
            failed_safe: false,
            prev_high: (0, 0),
            pump_off: 0,
        }
    }

    pub fn set_settings(&mut self, low: Depth, high: Depth, calibration: Calibration) {
        self.low_point = low;
        self.high_point = high;
        self.calibration = calibration;
        // reset failsafes in a round about way
        self.failed_safe = false;
        self.timed_out_tick_s = 0;
    }

    pub fn status(&mut self) -> bool { self.on_tick_s > 0 }

    pub fn tick(&mut self,  tick_s: u64, pump_pin: &mut GpioPin) -> io::Result<()> {
        if self.failed_safe { return Ok(()); }

        if self.timed_out_tick_s > 0 && self.timed_out_tick_s + TIMEOUT_PERIOD > tick_s {
            return Ok(());
        }
        let depth = self.depth_signal.sample();
        info!("Depth settings: {:?}-{:?}. Current: {:?}", self.low_point, self.high_point, depth);

        if self.pump_off > 0 && self.pump_off + 60*5 < tick_s {
            self.pump_off = 0;
            info!("Toggling pump back on");
            try!(pump_pin.turn_on());
        }
        if depth < self.low_point && self.on_tick_s == 0 {
            // TODO: we can sometimes get into this state after a timeout, need to figure out how
            // to fix it, or alert on it
            if depth < self.low_point - 20 {
                error!("Water level too far below bottom point, please manually fill the tank. This is to avoid overflows when the sensor isn't attached properly, during water changes, etc.");
                self.pump_off = tick_s;
                try!(pump_pin.turn_off());
                return Ok(());
            } 
            if self.prev_high.0 > 0 {
                let evap_per_hour = ((self.prev_high.1 - depth) as f32 / (tick_s - self.prev_high.0) as f32 / 60.0 / 60.0) as Depth;
                if evap_per_hour > self.calibration.max_evap_per_hour() {
                    warn!("Exceeded max_evap_per_hour, stopping. Actual steps: {}. Max: {}\n", evap_per_hour, self.calibration.max_evap_per_hour());
                    self.failed_safe = true;
                    return Ok(());
                }
            }
            if self.recent_high.sample() - depth > 20 {
                warn!("Quick drop in depth, waiting...");
                return Ok(());
            } 

            info!("Water level too low, toggling on: {:?}", depth);
            self.on_tick_s = tick_s;
            self.pin.turn_on() 
        // TODO: make configureable based on GPH of pump and estimated high - low
        } else if depth >= self.high_point { 
            info!("Water level too high, toggling off: {:?}", depth);
            self.on_tick_s = 0;
            self.prev_high = (tick_s, depth);
            self.pin.turn_off()
        } else if self.on_tick_s > 0 && tick_s - self.on_tick_s > self.calibration.run_time_s(self.low_point, self.high_point)  { 
            // TODO: alert should happen if the pump times out
            info!("Timed out depth after calculated seconds ({:?}), toggling off: {:?}", 
                     self.calibration.run_time_s(self.low_point, self.high_point), depth);
            self.on_tick_s = 0;
            self.timed_out_tick_s = tick_s;
            self.prev_high = (tick_s, depth);
            self.pin.turn_off()
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use carboxyl::Sink;

    #[test]
    pub fn run_time() {
        let calibration = Calibration {
            low: 60, 
            high: 120,
            high_inches: 6.0, // 10 steps per inch
            tank_surface_area: 100,
            pump_gph: 5.0, // .2 gpm 
            tank_volume: 10.0,
        };

        // raise 3 inches (30 steps / 10 steps per inch)
        assert_eq!(calibration.run_time_s(70, 100), 935);
    }
}
