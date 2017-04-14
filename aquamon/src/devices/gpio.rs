use std::io as io;
use sysfs_gpio::{Pin, Direction};
use sysfs_gpio::Error as GpioError;
use std::error::Error;
use std::io::ErrorKind;

pub struct GpioPin {
    pin: Pin,
}

fn from_gpio_error(e: GpioError) -> io::Error {
    match e {
        GpioError::Io(err) => { println!("{:#?}", err); err },
        GpioError::Unexpected(str) => io::Error::new(io::ErrorKind::Other, str),
        GpioError::InvalidPath(str) => io::Error::new(io::ErrorKind::NotFound, str)
    }
}


impl GpioPin {
    pub fn new(index: u64) -> io::Result<GpioPin> {
        let pin = Pin::new(index);
        pin.export().and_then(|_| pin.set_direction(Direction::Out))
                    .map(|_| GpioPin { pin: pin } )
                    .or_else(|f| {
                        println!("{:#?}", f);
                        Err(::std::io::Error::new(ErrorKind::Other, f.description()))
                    })
    }

    pub fn turn_on(&mut self) -> io::Result<()> {
        self.pin.set_value(1).map_err(from_gpio_error)
    }

    pub fn turn_off(&mut self) -> io::Result<()> {
        self.pin.set_value(0).map_err(from_gpio_error)
    }

    pub fn status(&mut self) -> io::Result<bool> {
        self.pin.get_value().map(|val| val != 0).map_err(from_gpio_error)
    }
}

