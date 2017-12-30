mod gpio;
mod avr_controller;
#[allow(dead_code)]
mod ph_monitor;

pub use self::gpio::GpioPin;
pub use self::ph_monitor::PhConfig;
use self::ph_monitor::PhMonitor;

use self::avr_controller::AvrController;
use ::uom::temp::*;

use std::io::ErrorKind;
use std::io as io;

use carboxyl::{Sink,Stream};

pub type Depth = u16;
#[allow(dead_code)]
pub type pH = f32;
pub type Humidity = f32;

pub struct Devices {
    avr_controller: AvrController,
    ph_monitor: PhMonitor,
    temp_sink: Sink<Temperature<F>>,
    depth_sink: Sink<Depth>,
    air_temp_sink: Sink<Temperature<F>>,
    humidity_sink: Sink<Humidity>,
    ph_sink: Sink<pH>,
    last_intensities: [u8; 6],
}

impl Devices {
    pub fn new(i2c_device_id: u8, ph_i2c_device_id: u8) -> io::Result<Devices> {
        let avr_controller = try!(AvrController::new(i2c_device_id));
        let ph_monitor = try!(PhMonitor::new(ph_i2c_device_id, PhConfig::default()));

        Ok(Devices {
            avr_controller: avr_controller,
            ph_monitor: ph_monitor,
            temp_sink: Sink::new(),
            depth_sink: Sink::new(),
            air_temp_sink: Sink::new(), 
            humidity_sink: Sink::new(),
            ph_sink: Sink::new(),
            last_intensities: [255_u8; 6], // initialize to high values so we ramp down by default
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

            // TODO: when re-enabling, remove dead code warning at top of this file
            // let ph = try!(self.ph_monitor.get_ph());

            self.ph_sink.send(10.0);
        }

        if ticks % 1000 == 0 {
            let depth = try!(self.avr_controller.get_depth());
            info!("Got depth: {:?}", depth);
            self.depth_sink.send(depth);
        }

        Ok(())
    }

    pub fn set_intensities(&mut self, values: &[u8; 6]) -> io::Result<()> { 
        // check so we don't spam the i2c bus
        if self.last_intensities != *values {
            self.last_intensities = values.clone();
            info!("Sending updated intensities: {:?}", values);
            self.avr_controller.set_intensities(values) 
        } else {
            Ok(())
        }
    }

    pub fn temp_stream(&self) -> Stream<Temperature<F>> { 
        self.temp_sink.stream()
    }
    pub fn depth_stream(&self) -> Stream<Depth> { self.depth_sink.stream() }
    pub fn air_temp_stream(&self) -> Stream<Temperature<F>> { self.air_temp_sink.stream() }
    pub fn humidity_stream(&self) -> Stream<Humidity> { self.humidity_sink.stream() }
    pub fn ph_stream(&self) -> Stream<pH> { self.ph_sink.stream() }
}

pub struct PiGpio {
    owners: [(u64,bool); 7],
}

impl PiGpio {
    pub fn new() -> PiGpio {
        PiGpio {
            // these are the exported pins we reserved for IO
            owners: [(17, false), (27, false), (22, false), (5, false), (6, false), (13, false), (26, false)]
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

