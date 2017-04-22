extern crate i2cdev;
extern crate aquamon;
extern crate byteorder;
extern crate aquamon_server;
extern crate chrono;
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate carboxyl;

use std::thread;
use std::io;
use std::io::{Read, Write};
use std::fs::{OpenOptions, File};
use std::error::Error;
use std::sync::mpsc::{channel, TryRecvError, Receiver};
use std::sync::{RwLock, Arc};
use chrono::{Local,NaiveTime};
use carboxyl::Signal;

// logging
use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;

use aquamon::uom::temp::Temperature;
use aquamon::devices::{Devices,Depth};
use aquamon::controller::{AquariumController, Calibration};
use aquamon::controller::schedule::{Schedule, ScheduleLeg};
use aquamon::controller::Status;

use aquamon_server::server::Status as StatusDto;
use aquamon_server::server::LightingSchedule as ScheduleDto;
use aquamon_server::server::Config as ConfigDto;
use aquamon_server::server::{Settings, Commands, LightSettings, TemperatureSettings, DepthSettings, DepthSettingsMaintain, DepthSettingsDepthValues};

const TICK_MS: u16 = 10;

fn main() {
    init_logging();
    let mut settings_dto = load_settings().unwrap_or(Settings {
        temperature_settings: TemperatureSettings { min: 79.5, max: 80.5 },
        depth_settings: DepthSettings { 
            maintainRange: DepthSettingsMaintain { low: 0, high: 1},
            depthValues: DepthSettingsDepthValues { low: 0, high: 4096, highInches: 10.0, tankSurfaceArea: 17*10, pumpGph: 50.0 }
        },
        lighting_schedule: ScheduleDto {
            schedule: vec![
                LightSettings { intensity: 0, intensities: [0_u8; 6], startTime: "09:00".to_string() },
                LightSettings { intensity: 0, intensities: [0_u8; 6], startTime: "17:00".to_string() },
            ]
        }
    });

    let (status_lock, rx_live, rx_commands) = start_server(&settings_dto);
    let mut devices = Devices::new(1).unwrap();
    let temp = Temperature::in_f(80.0);
    let temp_signal = devices.temp_stream()
        .fold((temp, temp, temp, temp), |(b,c,d,_), a| (a,b,c,d))
        .map(|(a,b,c,d)| ((a + b + c + d).value() / 4.0 * 10.0).round() as f32 / 10.0);
    let air_temp_signal = devices.air_temp_stream()
        .fold((temp, temp, temp, temp), |(b,c,d,_), a| (a,b,c,d))
        .map(|(a,b,c,d)| ((a + b + c + d).value() / 4.0 * 10.0).round() as f32 / 10.0);
    let humidity_signal = devices.humidity_stream()
        .fold((50.0, 50.0, 50.0, 50.0), |(b,c,d,_), a| (a,b,c,d))
        .map(|(a,b,c,d)| ((a + b + c + d) / 4.0 * 10.0).round() as f32 / 10.0);
    let depth_signal = devices.depth_stream().hold(61 * 4);
    let mut controller = AquariumController::new(Schedule::default(), 
                                                 Temperature::in_f(settings_dto.temperature_settings.min),
                                                 Temperature::in_f(settings_dto.temperature_settings.max),
                                                 settings_dto.depth_settings.maintainRange.low,
                                                 settings_dto.depth_settings.maintainRange.high,
                                                 Calibration {
                                                     low: settings_dto.depth_settings.depthValues.low,
                                                     high: settings_dto.depth_settings.depthValues.high,
                                                     high_inches: settings_dto.depth_settings.depthValues.highInches,
                                                     tank_surface_area: settings_dto.depth_settings.depthValues.tankSurfaceArea,
                                                     pump_gph: settings_dto.depth_settings.depthValues.pumpGph,
                                                 }, devices.temp_stream(), devices.depth_stream()); 

    let mut i:u64 = 0;
    loop {
        match rx_live.try_recv() {
            Ok(config_dto) => {
                let lights = config_dto.lights;
                let leg = ScheduleLeg { 
                    intensity: lights.intensity,
                    intensities: lights.intensities,
                    start_time: NaiveTime::from_hms(0,0,0)
                };

                match controller.live_mode(&mut devices, leg, 1) {
                    Err(err) => {
                        error!("Failed to write: {:?}. Need to retry this operation until it succeeds", err);
                    }, 
                    Ok(_) => {
                        trace!("Wrote intensities");
                    }
                }
            }, 
            Err(err) if err == TryRecvError::Disconnected => {
                panic!("Web server disconnected!");
            },
            _ => (),
        }

        match rx_commands.try_recv() {
            Ok(commands) => {
                match commands.lighting_schedule {
                    Some(schedule_dto) => {
                        let schedule = Schedule::new(schedule_dto.schedule.iter().map(|l| ScheduleLeg {
                            intensity: l.intensity,
                            intensities: l.intensities,
                            start_time: NaiveTime::parse_from_str(&l.startTime, "%H:%M").unwrap()
                        }).collect());

                        controller.schedule_updated(schedule);

                        settings_dto.lighting_schedule = schedule_dto;
                    }, 
                    None => {}
                }
                match commands.temperature_settings {
                    Some(temperature_settings) => {
                        controller.set_temp_range(
                            Temperature::in_f(temperature_settings.min),
                            Temperature::in_f(temperature_settings.max));
                        settings_dto.temperature_settings = temperature_settings;
                    },
                    None => {}
                }
                match commands.depth_settings {
                    Some(depth_settings) => {
                        controller.set_depth_settings(depth_settings.maintainRange.low, depth_settings.maintainRange.high,
                                                      Calibration {
                                                          low: settings_dto.depth_settings.depthValues.low,
                                                          high: settings_dto.depth_settings.depthValues.high,
                                                          high_inches: settings_dto.depth_settings.depthValues.highInches,
                                                          tank_surface_area: settings_dto.depth_settings.depthValues.tankSurfaceArea,
                                                          pump_gph: settings_dto.depth_settings.depthValues.pumpGph,
                                                      });
                        settings_dto.depth_settings = depth_settings;
                    },
                    None => {}
                }
                save_settings(&settings_dto);
            }, 
            Err(err) if err == TryRecvError::Disconnected => {
                panic!("Web server disconnected!");
            },
            _ => (),
        }

        i += 1;
        match devices.tick(i * TICK_MS as u64) {
            Ok(()) => {
                let mut status = status_lock.write().unwrap();
                status.currentTempF = temp_signal.sample();
                status.depth = depth_signal.sample();
                status.airTempF = air_temp_signal.sample();
                status.humidity = humidity_signal.sample();
            },
            Err(err) => error!("error ticking devices: {:?}", err)
        }
        match controller.tick(&mut devices, i * TICK_MS as u64) {
            Err(err) => error!("Failed to tick controller. Need to add retry logic. {:?}", err),
            Ok(_) => {}
        }

        if i % (30000 / TICK_MS as u64) == 0 {
            let status = controller.status();
            if let Err(result) = write_csv(&temp_signal, &depth_signal, &air_temp_signal, &humidity_signal, status) {
                error!("Could not write csv data: {}", result);
            }
        }

        thread::sleep(std::time::Duration::from_millis(TICK_MS as u64));
    }
}

fn start_server(settings: &Settings) -> (Arc<RwLock<StatusDto>>, Receiver<ConfigDto>, Receiver<Commands>) {
    let (tx, rx) = channel();
    let status = StatusDto { currentTempF: 0.0, depth: 0, airTempF: 0.0, humidity: 0.0 };
    let status_lock = Arc::new(RwLock::new(status));
    let status_return = status_lock.clone();
    let (tx_c, rx_c) = channel();
    
    let settings_for_server = settings.clone();

    thread::spawn(move || { aquamon_server::server::start(tx, status_lock, tx_c, settings_for_server); });
    trace!("started!");

    (status_return, rx, rx_c)
}

fn write_csv(temp: &Signal<f32>, depth: &Signal<Depth>, air_temp: &Signal<f32>, humidity: &Signal<f32>, status: Status) -> io::Result<()> {
    let file_opened = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("history.csv");

    file_opened.and_then(|mut file| {
        file.write_all(format!("{},{},{},{},{},{},{},{}\n", 
                           Local::now().format("%Y-%m-%dT%H:%M:%S%z"), 
                           temp.sample(),
                           depth.sample(),
                           status.heater_on,
                           status.ato_pump_on,
                           status.cooler_on,
                           air_temp.sample(),
                           humidity.sample()).as_bytes())
            .and_then(|_| file.sync_data())
    })
}

fn load_settings() -> io::Result<Settings> {
    let mut file = try!(File::open("settings.json"));
    let mut s = String::new();
    try!(file.read_to_string(&mut s));
    Ok(serde_json::from_str(&s).unwrap())
}

fn save_settings(config: &Settings) {
    let file_opened = OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .open("settings.json");

    let result = file_opened.and_then(|mut file| {
        let json_string = serde_json::to_string(config).unwrap();
        // can safely ignore as it only fails if not opened for writing
        file.set_len(0).unwrap();
        file.write_all(json_string.as_bytes())
    });
    
    if let Err(e) = result {
        error!("Could not write to settings file {}", e.description());
    } else {
        info!("settings written");
    }
}

fn init_logging() {
    let format = |record: &LogRecord| {
        format!("{} - {}", record.level(), record.args())
    };
    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Error);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }
    builder.init().unwrap();
}
