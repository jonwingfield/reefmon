mod gpio;
pub use self::gpio::GpioPin;

use ::uom::temp::*;

use std::io::ErrorKind;
use std::io as io;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

use byteorder::{ByteOrder, BigEndian};


pub struct PiGpio {
    owners: [(u64,bool); 4],
}

impl PiGpio {
    pub fn new() -> PiGpio {
        PiGpio {
            // these are the exported pins we reserved for IO
            owners: [(17, false), (27, false), (22, false), (5, false)]
        }
    }

    pub fn take_pin(&mut self, index: usize) -> io::Result<GpioPin> {
        let owner = self.owners[index];
        if owner.1 {
            return Err(io::Error::new(ErrorKind::AlreadyExists, "Port in use"));
        }
        // assign the owner
        self.owners[index] = (owner.0, true);
        GpioPin::new(owner.0)
    }
}

pub struct AvrController {
    device: LinuxI2CDevice,
}

const CONTROLLER_SLAVE_ADDR: u16 = 0x32;
const AQ_CMD_SETCHANNELS: u8 = 0x11;
const AQ_CMD_GET_TEMP: u8 = 0x12;
const AQ_CMD_GET_DEPTH: u8 = 0x13;

impl AvrController {
    pub fn new(i2c_device_id: u8) -> Result<AvrController, LinuxI2CError> {
        let device = try!(LinuxI2CDevice::new("/dev/i2c-".to_string() + &i2c_device_id.to_string(), CONTROLLER_SLAVE_ADDR));

        Ok(AvrController { 
            device: device
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

    pub fn get_depth(&mut self) -> io::Result<u8> {
        let mut depth = [0_u8; 2];
        try!(self.device.write(&[AQ_CMD_GET_DEPTH]));
        try!(self.device.read(&mut depth));

        try!(check_crc(&depth));

        Ok(depth[0])
    }

    pub fn set_intensities(&mut self, values: &[u8]) -> Result<(), LinuxI2CError> {
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

