use std::io as io;
use sysfs_gpio::{Pin, Direction};
use sysfs_gpio::Error as GpioError;
use std::error::Error;
use std::io::ErrorKind;

pub struct GpioPin {
    pin: Pin,
    active_low: bool,
}

fn from_gpio_error(e: GpioError) -> io::Error {
    match e {
        GpioError::Io(err) => { error!("{:#?}", err); err },
        GpioError::Unexpected(str) => io::Error::new(io::ErrorKind::Other, str),
        GpioError::InvalidPath(str) => io::Error::new(io::ErrorKind::NotFound, str)
    }
}


impl GpioPin {
    pub fn new(index: u64, active_low: bool) -> io::Result<GpioPin> {
        let pin = Pin::new(index);
        pin.export().and_then(|_| pin.set_direction(Direction::Out))
                    .and_then(|_| {
                        if active_low {
                            pin.set_value(1)
                        } else {
                            Ok(())
                        }
                    })
                    .map(|_| GpioPin { pin: pin, active_low: active_low } )
                    .or_else(|f| {
                        error!("{:#?}", f);
                        Err(::std::io::Error::new(ErrorKind::Other, f.description()))
                    })
    }

    pub fn set(&mut self, value: bool) -> io::Result<()> {
        self.pin.set_value(if value ^ self.active_low { 1 } else { 0 }).map_err(from_gpio_error)
    }

    pub fn turn_on(&mut self) -> io::Result<()> {
        self.pin.set_value(if self.active_low { 0 } else { 1 }).map_err(from_gpio_error)
    }

    pub fn turn_off(&mut self) -> io::Result<()> {
        self.pin.set_value(if self.active_low { 1 } else { 0 }).map_err(from_gpio_error)
    }

    pub fn status(&mut self) -> io::Result<bool> {
        self.pin.get_value().map(|val| (self.active_low && val == 0) || (!self.active_low && val != 0)).map_err(from_gpio_error)
    }
}

