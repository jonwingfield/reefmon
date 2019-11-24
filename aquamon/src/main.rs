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
use std::process::Command;
use chrono::{Local,NaiveTime};
use carboxyl::Signal;

// logging
use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;

use aquamon::uom::temp::Temperature;
use aquamon::devices::{Devices,Depth};
use aquamon::controller::{AquariumController, Calibration, Dose, TemperatureRange};
use aquamon::controller::schedule::{Schedule, ScheduleLeg};
use aquamon::controller::Status;

use aquamon_server::server::Status as StatusDto;
use aquamon_server::server::LightingSchedule as ScheduleDto;
use aquamon_server::server::{Settings, Commands, LightSettings, TemperatureSettings, TemperatureRangeSettings, DepthSettings, DepthSettingsMaintain, DepthSettingsDepthValues, LiveModeSettings, DoserSettings};

use aquamon::alerting::alert;

const TICK_MS: u64 = 10;

fn main() {
    init_logging();
    let mut settings_dto = load_settings().unwrap_or(Settings {
        temperature_settings: TemperatureSettings { 
            heater: TemperatureRangeSettings { min: 79.5, minTime: "07:00".to_string(), max: 79.5, maxTime: "16:00".to_string() },
            cooler: TemperatureRangeSettings { min: 80.5, minTime: "07:00".to_string(), max: 80.5, maxTime: "16:00".to_string() },
        },
        depth_settings: DepthSettings { 
            maintainRange: DepthSettingsMaintain { low: 0, high: 1},
            depthValues: DepthSettingsDepthValues { low: 0, high: 4096, highInches: 10.0, tankSurfaceArea: 17*10, tankVolume: 10.0, pumpGph: 50.0 }
        },
        lighting_schedule: ScheduleDto {
            schedule: vec![
                LightSettings { intensity: 0, intensities: [0_u8; 6], startTime: "09:00".to_string() },
                LightSettings { intensity: 0, intensities: [0_u8; 6], startTime: "17:00".to_string() },
            ]
        },
        doser_settings: DoserSettings {
            pumpRateMlMin: 1.1,
            schedule: vec![],
            doseAmountMl: 1.1,
            doseRangeStart: 7,
            doseRangeEnd: 18,
        }
    });

    let mut i:u64 = 0;

    let (status_lock, rx_live, rx_commands) = start_server(&settings_dto);
    let mut devices = Devices::new(1, 1).unwrap();
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
    let ph_signal = devices.ph_stream().hold(8.0);
    let depth_signal = devices.depth_stream().hold(61 * 4);
    let mut controller = AquariumController::new(map_schedule(&settings_dto.lighting_schedule), 
                                                 map_temperature_range(&settings_dto.temperature_settings.heater),
                                                 map_temperature_range(&settings_dto.temperature_settings.cooler),
                                                 settings_dto.depth_settings.maintainRange.low,
                                                 settings_dto.depth_settings.maintainRange.high,
                                                 Calibration {
                                                     low: settings_dto.depth_settings.depthValues.low,
                                                     high: settings_dto.depth_settings.depthValues.high,
                                                     high_inches: settings_dto.depth_settings.depthValues.highInches,
                                                     tank_surface_area: settings_dto.depth_settings.depthValues.tankSurfaceArea,
                                                     tank_volume: settings_dto.depth_settings.depthValues.tankVolume,
                                                     pump_gph: settings_dto.depth_settings.depthValues.pumpGph,
                                                 }, devices.temp_stream(), devices.depth_stream(),
                                                 settings_dto.doser_settings.pumpRateMlMin);
    loop {
        match rx_live.try_recv() {
            Ok(config_dto) => {
                let lights = config_dto.lights;
                let leg = ScheduleLeg { 
                    intensity: lights.intensity,
                    intensities: lights.intensities,
                    start_time: NaiveTime::from_hms(0,0,0)
                };

                controller.live_mode(i * TICK_MS, leg);
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
                        let schedule = map_schedule(&schedule_dto); 
                        controller.schedule_updated(schedule);

                        settings_dto.lighting_schedule = schedule_dto;
                    }, 
                    None => {}
                }
                match commands.temperature_settings {
                    Some(temperature_settings) => {
                        controller.set_temp_range(
                            map_temperature_range(&temperature_settings.heater),
                            map_temperature_range(&temperature_settings.cooler));
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
                                                          tank_volume: settings_dto.depth_settings.depthValues.tankVolume,
                                                      });
                        settings_dto.depth_settings = depth_settings;
                    },
                    None => {}
                }
                if let Some(toggles) = commands.toggles {
                    if let Err(err) = controller.enable_pump(toggles.pump, i * TICK_MS) {
                        error!("Error enabling pump: {}", err);
                    }
                }
                if let Some(viewing_mode) = commands.viewing_mode {
                    let lights = viewing_mode.lights;
                    let leg = ScheduleLeg { 
                        intensity: lights.intensity,
                        intensities: lights.intensities,
                        start_time: NaiveTime::from_hms(0,0,0)
                    };
                    controller.set_viewing_mode(viewing_mode.on, i * TICK_MS, leg);
                }
                if let Some(doser_settings) = commands.doser_settings {
                    controller.set_doser_settings(
                        doser_settings.pumpRateMlMin,
                        doser_settings.schedule.iter().map(|leg| Dose {
                            dose_amount_ml: leg.doseAmountMl,
                            start_time: parse_time(&leg.startTime),
                        }).collect());
                    settings_dto.doser_settings = doser_settings;
                }
                if commands.garage_door_opener.is_some() {
                    open_garage_door();
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
                let controller_status = controller.status();
                status.currentTempF = temp_signal.sample();
                status.depth = depth_signal.sample();
                status.airTempF = air_temp_signal.sample();
                status.humidity = humidity_signal.sample();
                status.pH = ph_signal.sample();
                status.heater_on = controller_status.heater_on;
                status.ato_pump_on = controller_status.ato_pump_on;
                status.cooler_on = controller_status.cooler_on;
                status.pump_on = controller_status.pump_on;
            },
            Err(err) => error!("error ticking devices: {:?}", err)
        }
        match controller.tick(&mut devices, i * TICK_MS as u64) {
            Err(err) => error!("Failed to tick controller. Need to add retry logic. {:?}", err),
            Ok(_) => {}
        }

        if i % (30000 / TICK_MS as u64) == 0 {
            let status = controller.status();
            if let Err(result) = write_csv(&temp_signal, &depth_signal, &air_temp_signal, &humidity_signal, &ph_signal, &status) {
                error!("Could not write csv data: {}", result);
            }
            if let Err(result) = alert(&status.alerts) {
                error!("Error sending alerts: {:?}", result);
            }
        }

        thread::sleep(std::time::Duration::from_millis(TICK_MS as u64));
    }
}

fn parse_time(time: &String) -> NaiveTime {
    NaiveTime::parse_from_str(&time, "%H:%M").unwrap()
}

fn map_schedule(schedule_dto: &ScheduleDto) -> Schedule {
    Schedule::new(schedule_dto.schedule.iter().map(|l| ScheduleLeg {
        intensity: l.intensity,
        intensities: l.intensities,
        start_time: NaiveTime::parse_from_str(&l.startTime, "%H:%M").unwrap()
    }).collect())
}

fn map_temperature_range(settings: &TemperatureRangeSettings) -> TemperatureRange { 
    TemperatureRange { 
        min: Temperature::in_f(settings.min),
        min_time: parse_time(&settings.minTime),
        max: Temperature::in_f(settings.max),
        max_time: parse_time(&settings.maxTime),
    }
}

fn start_server(settings: &Settings) -> (Arc<RwLock<StatusDto>>, Receiver<LiveModeSettings>, Receiver<Commands>) {
    let (tx, rx) = channel();
    let status = StatusDto { currentTempF: 0.0, depth: 0, airTempF: 0.0, humidity: 0.0, pH: 0.0, heater_on: false, cooler_on: false, ato_pump_on: false, pump_on: false };
    let status_lock = Arc::new(RwLock::new(status));
    let status_return = status_lock.clone();
    let (tx_c, rx_c) = channel();
    
    let settings_for_server = settings.clone();

    thread::spawn(move || { aquamon_server::server::start(tx, status_lock, tx_c, settings_for_server); });
    trace!("started!");

    (status_return, rx, rx_c)
}

fn write_csv(temp: &Signal<f32>, depth: &Signal<Depth>, air_temp: &Signal<f32>, humidity: &Signal<f32>, ph_signal: &Signal<f32>, status: &Status) -> io::Result<()> {
    let file_opened = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open("history.csv");

    file_opened.and_then(|mut file| {
        file.write_all(format!("{},{},{},{},{},{},{},{},{}\n", 
                           Local::now().format("%Y-%m-%dT%H:%M:%S%z"), 
                           temp.sample(),
                           depth.sample(),
                           status.heater_on,
                           status.ato_pump_on,
                           status.cooler_on,
                           air_temp.sample(),
                           humidity.sample(), 
                           ph_signal.sample()).as_bytes())
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

fn open_garage_door() {
    info!("Opening garage door");
    match Command::new("sh")
        .arg("-c")
        .arg("ssh -i /home/pi/.ssh/id_rsa_pi1 pi@pi1 './open_garage_door.sh'")
        .output() {
            Err(e) => error!("Could not open garage door: {:?}", e),
            Ok(output) => info!("Opening garage door: {:?}", output)
    }
}
