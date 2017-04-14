extern crate i2cdev;
extern crate aquamon;
extern crate byteorder;
extern crate aquamon_server;
extern crate chrono;
extern crate serde_json;

use std::thread;
use std::io;
use std::io::{Read, Write};
use std::fs::{OpenOptions, File};
use std::error::Error;
use std::sync::mpsc::{channel, TryRecvError, Receiver};
use std::sync::{RwLock, Arc};
use chrono::NaiveTime;

use aquamon::uom::temp::Temperature;
use aquamon::devices::AvrController;
use aquamon::controller::{AquariumController, Calibration};
use aquamon::controller::schedule::{Schedule, ScheduleLeg};

use aquamon_server::server::Status as StatusDto;
use aquamon_server::server::LightingSchedule as ScheduleDto;
use aquamon_server::server::Config as ConfigDto;
use aquamon_server::server::{Settings, Commands, LightSettings, TemperatureSettings, DepthSettings, DepthSettingsMaintain, DepthSettingsDepthValues};

const TICK_MS: u16 = 10;

fn main() {
    let mut settings_dto = load_settings().unwrap_or(Settings {
        temperature_settings: TemperatureSettings { setPoint: 79.5 },
        depth_settings: DepthSettings { 
            maintainRange: DepthSettingsMaintain { low: 0, high: 1},
            depthValues: DepthSettingsDepthValues { low: 0, high: 255, highInches: 10.0, tankSurfaceArea: 17*10 }
        },
        lighting_schedule: ScheduleDto {
            schedule: vec![
                LightSettings { intensity: 0, intensities: [0_u8; 6], startTime: "09:00".to_string() },
                LightSettings { intensity: 0, intensities: [0_u8; 6], startTime: "17:00".to_string() },
            ]
        }
    });

    let (status_lock, rx_live, rx_commands) = start_server(&settings_dto);
    let avr_controller = AvrController::new(1).unwrap();
    let mut controller = AquariumController::new(Schedule::default(), avr_controller, TICK_MS, 
                                                 Temperature::in_f(settings_dto.temperature_settings.setPoint),
                                                 settings_dto.depth_settings.maintainRange.low,
                                                 settings_dto.depth_settings.maintainRange.high,
                                                 Calibration {
                                                     low: settings_dto.depth_settings.depthValues.low,
                                                     high: settings_dto.depth_settings.depthValues.high,
                                                     highInches: settings_dto.depth_settings.depthValues.highInches,
                                                     tankSurfaceArea: settings_dto.depth_settings.depthValues.tankSurfaceArea,
                                                 }); 

    let mut i:u16 = 0;
    loop {
        match rx_live.try_recv() {
            Ok(config_dto) => {
                let lights = config_dto.lights;
                let leg = ScheduleLeg { 
                    intensity: lights.intensity,
                    intensities: lights.intensities,
                    start_time: NaiveTime::from_hms(0,0,0)
                };

                match controller.live_mode(leg, 1) {
                    Err(err) => {
                        println!("Failed to write: {:?}. Need to retry this operation until it succeeds", err);
                    }, 
                    Ok(_) => {
                        println!("Wrote intensities");
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
                        controller.set_temp_setpoint(Temperature::in_f(temperature_settings.setPoint));
                        settings_dto.temperature_settings = temperature_settings;
                    },
                    None => {}
                }
                match commands.depth_settings {
                    Some(depth_settings) => {
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

        i = i.overflowing_add(1_u16).0;
        if i % 300 == 0 {
            match controller.get_temp() {
                Ok(temp) => {
                    println!("Got temp: {:?}", temp.to_f().value()); 
                    let mut status = status_lock.write().unwrap();
                    status.currentTempF = temp.to_f().value();
                },
                Err(err) => println!("Error getting temp: {:?}", err),
            }
            match controller.get_depth() {
                Ok(depth) => {
                    println!("Got depth: {:?}", depth);
                    let mut status = status_lock.write().unwrap();
                    status.depth = depth;
                },
                Err(err) => println!("Error getting depth: {:?}", err),
            }
        }
        match controller.tick() {
            Err(err) => println!("Failed to tick controller. Need to add retry logic. {:?}", err),
            Ok(_) => {}
        }
        thread::sleep(std::time::Duration::from_millis(TICK_MS as u64));
    }
}

fn start_server(settings: &Settings) -> (Arc<RwLock<StatusDto>>, Receiver<ConfigDto>, Receiver<Commands>) {
    let (tx, rx) = channel();
    let status = StatusDto { currentTempF: 0.0, depth: 0 };
    let status_lock = Arc::new(RwLock::new(status));
    let status_return = status_lock.clone();
    let (tx_c, rx_c) = channel();
    
    let settings_for_server = settings.clone();

    thread::spawn(move || { aquamon_server::server::start(tx, status_lock, tx_c, settings_for_server); });
    println!("started!");

    (status_return, rx, rx_c)
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

    let result = file_opened.map(|mut file| {
        let json_string = serde_json::to_string(config).unwrap();
        file.write_all(json_string.as_bytes())
    });
    
    if let Err(e) = result {
        println!("Could not write to settings file {}", e.description());
    } else {
        println!("settings written");
    }
}
