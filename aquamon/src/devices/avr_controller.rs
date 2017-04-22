use ::uom::temp::*;

use std::io::ErrorKind;
use std::io as io;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice};
use super::Depth;

use byteorder::{ByteOrder, BigEndian};

pub struct AvrController {
    device: LinuxI2CDevice,
    depth_avg: [Depth; 10],
    depth_index: usize,
}

const CONTROLLER_SLAVE_ADDR: u16 = 0x32;
const AQ_CMD_SETCHANNELS: u8 = 0x11;
const AQ_CMD_GET_TEMP: u8 = 0x12;
const AQ_CMD_GET_DEPTH: u8 = 0x13;
const AQ_CMD_GET_AIR_TEMP_HUMIDITY: u8 = 0x14;

impl AvrController {
    pub fn new(i2c_device_id: u8) -> Result<AvrController, io::Error> {
        let device = try!(LinuxI2CDevice::new("/dev/i2c-".to_string() + &i2c_device_id.to_string(), CONTROLLER_SLAVE_ADDR));
        Ok(AvrController { 
            device: device,
            depth_avg: [0; 10],
            depth_index: 0,
        })
    }

    pub fn get_temp(&mut self) -> Result<Temperature<C>, io::Error> {
        let mut temp = [0_u8; 3];
        try!(self.device.write(&[AQ_CMD_GET_TEMP]));
        try!(self.device.read(&mut temp));

        let temp_c = (BigEndian::read_u16(&temp) as f32) * 0.0625;

        try!(check_crc(&temp));

        Ok(Temperature::in_c(temp_c))
    }

    pub fn get_air_temp_humidity(&mut self) -> io::Result<(Temperature<C>, f32)> {
        let mut temp = [0_u8; 5];
        try!(self.device.write(&[AQ_CMD_GET_AIR_TEMP_HUMIDITY]));
        try!(self.device.read(&mut temp));

        let temp_c = (BigEndian::read_u16(&temp) as f32) / 10.0;
        let humidity = (BigEndian::read_u16(&temp[2..4]) as f32) / 10.0;

        try!(check_crc(&temp));

        Ok((Temperature::in_c(temp_c), humidity))
    }

    pub fn get_depth(&mut self) -> io::Result<Depth> {
        let mut depth = [0_u8; 3];
        try!(self.device.write(&[AQ_CMD_GET_DEPTH]));
        try!(self.device.read(&mut depth));

        try!(check_crc(&depth));

        let result = BigEndian::read_u16(&depth);

        // TODO: refactor averaging code
        if self.depth_avg[0] == 0 {
            self.depth_avg = [result; 10];
        } else {
            self.depth_avg[self.depth_index] = result;
        }
        self.depth_index = if self.depth_index >= 9 { 0 } else { self.depth_index + 1 };

        Ok(self.depth_avg.iter().fold(0, |sum, i| sum + i) / 10)
    }

    pub fn set_intensities(&mut self, values: &[u8]) -> Result<(), io::Error> {
        let mut intensities = [0_u8; 7];
        intensities[0..6].copy_from_slice(values);
        intensities[6] = crc(&intensities);
        try!(self.device.smbus_write_block_data(AQ_CMD_SETCHANNELS, &intensities));
        Ok(())
    }
}

fn check_crc(values: &[u8]) -> io::Result<()> {
    let calculated_crc = crc(&values);
    if calculated_crc != values[values.len() - 1] {
        return Err(io::Error::new(ErrorKind::InvalidData, format!("Bad CRC: Expected {:?} Got {:?}", calculated_crc, values[values.len()-1])));
    }

    Ok(())
}

fn crc(values: &[u8]) -> u8 {
    let mut crc: u8 = 0xff;

    for j in 0..values.len() - 1 {
        crc ^= values[j];
        for _ in 0..8 {
            if crc & 0x80 != 0 
            {
                crc <<= 1;
                crc ^= 0x07;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

