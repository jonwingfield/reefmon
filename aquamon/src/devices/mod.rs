mod gpio;
mod avr_controller;

pub use self::gpio::GpioPin;

use self::avr_controller::AvrController;
use ::uom::temp::*;

use std::io::ErrorKind;
use std::io as io;

use carboxyl::{Sink,Stream};

pub type Depth = u16;
pub type Humidity = f32;

pub struct Devices {
    avr_controller: AvrController,
    temp_sink: Sink<Temperature<F>>,
    depth_sink: Sink<Depth>,
    air_temp_sink: Sink<Temperature<F>>,
    humidity_sink: Sink<Humidity>,
}

impl Devices {
    pub fn new(i2c_device_id: u8) -> io::Result<Devices> {
        let avr_controller = try!(AvrController::new(i2c_device_id));

        Ok(Devices {
            avr_controller: avr_controller,
            temp_sink: Sink::new(),
            depth_sink: Sink::new(),
            air_temp_sink: Sink::new(), 
            humidity_sink: Sink::new(),
        })
    }

    pub fn tick(&mut self, ticks: u64) -> io::Result<()> {
        // temp is only updated by the micro every 5 seconds + 1 second conversion time
        if ticks % 6500 == 0 {
            let temp = try!(self.avr_controller.get_temp()).to_f();
            info!("Got temp: {:?}", temp.value());
            self.temp_sink.send(temp);

            let (air_temp, humidity) = try!(self.avr_controller.get_air_temp_humidity());
            info!("Got air temp: {:?} and humidity: {:?}", air_temp.to_f().value(), humidity);
            self.air_temp_sink.send(air_temp.to_f());
            self.humidity_sink.send(humidity);
        }

        if ticks % 1000 == 0 {
            let depth = try!(self.avr_controller.get_depth());
            info!("Got depth: {:?}", depth);
            self.depth_sink.send(depth);
        }

        Ok(())
    }

    pub fn set_intensities(&mut self, values: &[u8]) -> io::Result<()> { 
        self.avr_controller.set_intensities(values) 
    }

    pub fn temp_stream(&self) -> Stream<Temperature<F>> { 
        self.temp_sink.stream()
    }
    pub fn depth_stream(&self) -> Stream<Depth> { self.depth_sink.stream() }
    pub fn air_temp_stream(&self) -> Stream<Temperature<F>> { self.air_temp_sink.stream() }
    pub fn humidity_stream(&self) -> Stream<Humidity> { self.humidity_sink.stream() }
}

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

    pub fn take_pin(&mut self, index: usize, active_low: bool) -> io::Result<GpioPin> {
        let owner = self.owners[index];
        if owner.1 {
            return Err(io::Error::new(ErrorKind::AlreadyExists, "Port in use"));
        }
        // assign the owner
        self.owners[index] = (owner.0, true);
        GpioPin::new(owner.0, active_low)
    }
}

