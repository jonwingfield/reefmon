use std::io as io;

use super::pH;
use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice};

use byteorder::{ByteOrder, BigEndian};

pub struct PhMonitor {
    mcp3221: Mcp3221,
    ph_avg: [pH; 20],
    ph_index: usize,
    config: PhConfig,
}

pub struct PhConfig {
    pub ph7_cal: u16,
    pub ph10_cal: u16,
}

// TODO: setup these values
const SLAVE_ADDR: u16 = 0x4d;
const V_REF: f32 = 4.096;
const OPAMP_GAIN: f32 = 1.0;
// const IDEAL_SLOPE: f32 = 59.16;
const STEP_12BIT: f32 = 4096.0;

impl PhConfig {
    pub fn default() -> PhConfig {
        PhConfig {
            ph7_cal: 2048,
            ph10_cal: 1065, // manually calibrated
        }
    }

    //Temperature compensation can be added by providing the temp offset per degree
    //IIRC .009 per degree off 25c (temperature-25*.009 added pH@4calc)
    pub fn get_step(&self) -> f32 {
        // RefVoltage * our deltaRawpH / 12bit steps *mV in V / OP-Amp gain /pH step difference 10-7
        (V_REF * (self.ph10_cal as f32 - self.ph7_cal as f32) as f32) / STEP_12BIT * 1000.0 / OPAMP_GAIN / 3.0
    }
}

#[allow(non_snake_case)]
fn calc_ph(raw: u16, config: &PhConfig) -> pH {
    info!("Raw pH: {:?}, step: {:?}", raw, config.get_step());
    let mV: f32 = (raw as f32) / STEP_12BIT * V_REF * 1000.0;
    let temp: f32 = ((V_REF * config.ph7_cal as f32 / STEP_12BIT * 1000.0) - mV) / OPAMP_GAIN;
    let ph = 7.0 - (temp / config.get_step());
    info!("Calculated pH at {:?}", ph);
    if ph > 10.0 {
        warn!("pH greater than 10, truncating to 10");
        return 10.0;
    }
    ph
}

impl PhMonitor {
    pub fn new(i2c_device_id: u8, config: PhConfig) -> io::Result<PhMonitor> {
        let mcp3221 = try!(Mcp3221::new(i2c_device_id));
        Ok(PhMonitor {
            mcp3221: mcp3221,
            ph_avg: [8.0_f32; 20],
            ph_index: 0,
            config: config,
        })
    }

    pub fn get_ph(&mut self) -> io::Result<pH> {
        let raw: u16 = try!(self.mcp3221.sample());

        let ph = calc_ph(raw, &self.config);
        let avg = self.update_average_and_get(ph);
        info!("Got raw ph reading: {:?}, avg: {:?}", raw, avg);
        Ok((avg * 100.0).round() / 100.0)
    }

    pub fn update_average_and_get(&mut self, ph: pH) -> pH {
        self.ph_avg[self.ph_index] = ph;
        self.ph_index += 1;
        if self.ph_index >= self.ph_avg.len() { self.ph_index = 0; }
        self.ph_avg.iter().fold(0.0, |a,b| a+b) / self.ph_avg.len() as f32
    }
}

struct Mcp3221 {
    device: LinuxI2CDevice,
}

impl Mcp3221 {
    pub fn new(i2c_device_id: u8) -> io::Result<Mcp3221> {
        let device = try!(LinuxI2CDevice::new("/dev/i2c-".to_string() + &i2c_device_id.to_string(), SLAVE_ADDR));
        Ok(Mcp3221 {
            device: device
        })
    }

    pub fn get_value(&mut self) -> io::Result<u16> {
        let mut raw_value = [0_u8; 2];
        try!(self.device.read(&mut raw_value));

        Ok(BigEndian::read_u16(&raw_value) as u16)
    }

    pub fn sample(&mut self) -> io::Result<u16> {
        let values = [
            try!(self.get_value()) as u32,
            try!(self.get_value()) as u32,
            try!(self.get_value()) as u32,
            try!(self.get_value()) as u32,
            try!(self.get_value()) as u32];

        Ok((values.iter().fold(0, |a, b| a + b) / values.len() as u32) as u16)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn step_calculation() {
        let config = PhConfig::default();

        // should be very close to ideal of 59.16
        assert_eq!(config.get_step(), 59.174606);
    }

    #[test]
    pub fn step_calculation_with_drift() {
        let config = PhConfig {
            ph7_cal: 2020,
            ph10_cal: 2999,
        };

        assert_eq!(config.get_step(), 62.158733);
    }

    #[test]
    pub fn ph_calculation() {
        let config = PhConfig::default();

        // sanity check
        assert_eq!(calc_ph(2048+932, &config), 10.0);
        // now test a random sampling. We don't care about pH < 7 for our use
        assert_eq!(calc_ph(2102, &config), 7.17);
        assert_eq!(calc_ph(2500, &config), 8.45);
        assert_eq!(calc_ph(2842, &config), 9.56);
    }

    #[test]
    pub fn ph_calculation_with_drift() {
        let config = PhConfig {
            ph7_cal: 2020,
            ph10_cal: 2999,
        };

        // sanity check
        assert_eq!(calc_ph(2020, &config), 7.0);
        assert_eq!(calc_ph(2999, &config), 10.0);
        // now test a random sampling. We don't care about pH < 7 for our use
        assert_eq!(calc_ph(2102, &config), 7.25);
        assert_eq!(calc_ph(2500, &config), 8.47);
        assert_eq!(calc_ph(2842, &config), 9.52);
    }
}
