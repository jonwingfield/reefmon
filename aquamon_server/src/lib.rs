extern crate iron;
extern crate mount;
extern crate bodyparser;
extern crate router;
extern crate staticfile;
extern crate persistent;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate iron_compress;
extern crate urlencoded;

extern crate serde_json;

pub mod server {
    use iron::{headers, middleware, status};
    use iron::prelude::*;
    use iron::typemap::TypeMap;
    use iron_compress::GzipWriter;
    use urlencoded::UrlEncodedQuery;
    // use iron::{Iron, Request, Response, IronResult};

    use mount::Mount;
    use router::Router;
    use staticfile::Static;

    use std::path::Path;
    use std::vec::Vec;
    use std::fmt;

    use std::sync::{Mutex, RwLock, Arc};
    use std::sync::mpsc::Sender;
    use std::fs::File;
    use std::io::{Read, SeekFrom, Seek};
    use std::error::Error;
     
    use serde_json;

    use bodyparser;

    const MAX_BODY_LENGTH: usize = 1024 * 1024 * 10;

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LightSettings {
        pub intensities: [u8; 6],
        pub intensity: u8,
        pub startTime: String
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct LightingSchedule {
        pub schedule: Vec<LightSettings>,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LiveModeSettings {
        pub lights: LightSettings,
        pub on: bool,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Status {
        pub currentTempF: f32,
        pub depth: u16,
        pub airTempF: f32,
        pub humidity: f32,
        pub pH: f32,
        // pub timestamp: String,
        // TODO: map of on/off triggers
        // TODO: water level
        //
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct TemperatureSettings {
        pub heater: TemperatureRangeSettings,
        pub cooler: TemperatureRangeSettings,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct TemperatureRangeSettings {
        pub min: f32,
        pub minTime: String,
        pub max: f32,
        pub maxTime: String,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct DepthSettingsMaintain {
        pub low: u16,
        pub high: u16,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct DepthSettingsDepthValues {
        pub low: u16,
        pub high: u16,
        pub highInches: f32,
        pub tankSurfaceArea: u16,
        pub pumpGph: f32,
        pub tankVolume: f32,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct DepthSettings {
        pub maintainRange: DepthSettingsMaintain,
        pub depthValues: DepthSettingsDepthValues,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct DoserSettings {
        pub pumpRateMlMin: f32,
        pub schedule: Vec<Dose>,
        // Currently client-side only
        pub doseAmountMl: f32,
        pub doseRangeStart: i8,
        pub doseRangeEnd: i8,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct Dose {
        pub doseAmountMl: f32,
        pub startTime: String,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Toggles {
        pub pump: bool,
    }

    // Used to send commands to the main service
    #[derive(Default)]
    pub struct Commands {
        pub temperature_settings: Option<TemperatureSettings>,
        pub depth_settings: Option<DepthSettings>,
        pub lighting_schedule: Option<LightingSchedule>,
        pub toggles: Option<Toggles>,
        pub viewing_mode: Option<LiveModeSettings>,
        pub garage_door_opener: Option<()>,
        pub doser_settings: Option<DoserSettings>,
    }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub temperature_settings: TemperatureSettings,
    pub depth_settings: DepthSettings,
    pub lighting_schedule: LightingSchedule,
    pub doser_settings: DoserSettings,
}

// impl fmt::Display for Config {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "(Max {}, Min {}, Fan {} hours)", self.maxTempF, self.minTempF, self.fanDurationHours)
//     }
// }

pub fn start(tx_live: Sender<LiveModeSettings>, status_lock: Arc<RwLock<Status>>, tx_command: Sender<Commands>, settings: Settings) {
    let mut router = Router::new();
    let mut ra_router = Router::new();
    let mutex_live = Mutex::new(tx_live);
    let mutex_c = Arc::new(Mutex::new(tx_command));

    let lighting_lock = Arc::new(RwLock::new(settings.lighting_schedule));
    let (writer_lighting, mutex_lighting) = (lighting_lock.clone(), mutex_c.clone());
    router.get("/settings/lighting/schedule", move |_: &mut Request| {
        let schedule = lighting_lock.read().unwrap();

        Ok(Response::with((status::Ok, serde_json::to_string(&(*schedule)).unwrap())))

    }, "lighting_schedule");

    router.post("/settings/lighting/schedule", move |req: &mut Request| {
        let body = req.get::<bodyparser::Struct<LightingSchedule>>();
        match body {
            Ok(Some(schedule)) => {
                {
                    let tx_s = mutex_lighting.lock().unwrap();

                    tx_s.send(Commands { lighting_schedule: Some(schedule.clone()), ..Default::default() }).unwrap();
                }

                {
                    let mut c = writer_lighting.write().unwrap();
                    *c = schedule.clone();
                }

                Ok(Response::with((status::Ok, serde_json::to_string(&schedule).unwrap())))
            },
            // TODO: handle errors
            Ok(None) => {
                error!("Error");
                return Ok(Response::with(status::Ok));
            },
            Err(err) => {
                error!("Error: {:?}", err);
                return Ok(Response::with(status::Ok));
            },
        }
    }, "lighting_schedule");

    let temp_settings_lock = Arc::new(RwLock::new(settings.temperature_settings));
    let (writer_temp_settings, mutex_temp) = (temp_settings_lock.clone(), mutex_c.clone());
    router.get("/settings/temperature", move |_: &mut Request| {
        let temp_settings = temp_settings_lock.read().unwrap();

        Ok(Response::with((status::Ok, serde_json::to_string(&(*temp_settings)).unwrap())))
    }, "temperature_settings");

    router.post("/settings/temperature", move |req: &mut Request| {
        let body = req.get::<bodyparser::Struct<TemperatureSettings>>();

            match body {
                Ok(Some(temperature_settings)) => {
                    {
                        mutex_temp.lock().unwrap()
                            .send(Commands { temperature_settings: Some(temperature_settings.clone()), ..Default::default() })
                            .unwrap();
                    }
                     
                    { 
                        let mut c = writer_temp_settings.write().unwrap();
                        *c = temperature_settings;
                    }

                    Ok(Response::with((status::Ok, "".to_string())))
                },
                Ok(None) => { error!("Error"); Ok(Response::with(status::Ok)) },
                Err(err) => { error!("Error: {:?}", err); Ok(Response::with(status::Ok)) }
            }
        }, "temperature_settings");

        let depth_settings_lock = Arc::new(RwLock::new(settings.depth_settings));
        let (writer_depth_settings, mutex_depth) = (depth_settings_lock.clone(), mutex_c.clone());
        router.get("/settings/depth", move |_: &mut Request| {
            let depth_settings = depth_settings_lock.read().unwrap();

            Ok(Response::with((status::Ok, serde_json::to_string(&(*depth_settings)).unwrap())))
        }, "depth_settings");

        router.post("/settings/depth", move |req: &mut Request| {
            let body = req.get::<bodyparser::Struct<DepthSettings>>();

            match body {
                Ok(Some(depth_settings)) => {
                    {
                        mutex_depth.lock().unwrap()
                            .send(Commands { depth_settings: Some(depth_settings), ..Default::default() })
                            .unwrap();
                    }
                    {
                        let mut x = writer_depth_settings.write().unwrap();
                        *x = depth_settings.clone();
                    }

                    Ok(Response::with((status::Ok, "".to_string())))
                },
                Ok(None) => { error!("Error"); Ok(Response::with(status::Ok)) },
                Err(err) => { error!("Error: {:?}", err); Ok(Response::with(status::Ok)) }
            }
        }, "depth_settings");

        let doser_settings_lock = Arc::new(RwLock::new(settings.doser_settings));
        let (writer_doser_settings, mutex_doser) = (doser_settings_lock.clone(), mutex_c.clone());
        router.get("/settings/doser", move |_: &mut Request| {
            let doser_settings = doser_settings_lock.read().unwrap();

            Ok(Response::with(
                (status::Ok, serde_json::to_string(&(*doser_settings)).unwrap()))
            )
        }, "doser_settings");

        router.post("/settings/doser", move |req: &mut Request| {
            let body = req.get::<bodyparser::Struct<DoserSettings>>();
            
            match body {
                Ok(Some(doser_settings)) => {
                    {
                        mutex_doser.lock().unwrap()
                            .send(Commands { doser_settings: Some(doser_settings.clone()), ..Default::default() })
                            .unwrap();
                    }
                    {
                        let mut x = writer_doser_settings.write().unwrap();
                        *x = doser_settings;
                    }
                    Ok(Response::with((status::Ok, "".to_string())))
                },
                Ok(None) => { error!("Error"); Ok(Response::with(status::Ok)) },
                Err(err) => { error!("Error: {:?}", err); Ok(Response::with(status::Ok)) }
            }
        }, "doser_settings");

        router.post("/lighting/live", move |req: &mut Request| {
            let body = req.get::<bodyparser::Struct<LiveModeSettings>>();
            match body {
                Ok(Some(config)) => {
                    {
                        let tx = mutex_live.lock().unwrap();

                        tx.send(config.clone()).unwrap();
                    }

                    Ok(Response::with((status::Ok, serde_json::to_string(&config).unwrap())))
                },
                // TODO: handle errors
                Ok(None) => {
                    error!("Error");
                    return Ok(Response::with(status::Ok));
                },
                Err(_) => {
                    error!("Error");
                    return Ok(Response::with(status::Ok));
                },
            }
        }, "live");

        let mutex_toggles = mutex_c.clone();
        router.post("/toggles/", move |req: &mut Request| {
            let body = req.get::<bodyparser::Struct<Toggles>>();
            match body {
                Ok(Some(toggles)) => {
                    {
                        mutex_toggles.lock().unwrap()
                            .send(Commands { toggles: Some(toggles), ..Default::default() })
                            .unwrap();
                    }

                    Ok(Response::with((status::Ok, serde_json::to_string(&toggles).unwrap())))
                },
                // TODO: handle errors
                Ok(None) => {
                    error!("Error");
                    return Ok(Response::with(status::Ok));
                },
                Err(_) => {
                    error!("Error");
                    return Ok(Response::with(status::Ok));
                },
            }
        }, "toggles");

        let mutex_viewing_mode = mutex_c.clone();
        router.post("/viewingMode/", move |req: &mut Request| {
            let body = req.get::<bodyparser::Struct<LiveModeSettings>>();
            match body {
                Ok(Some(live_mode_settings)) => {
                    let ret = live_mode_settings.clone();
                    {
                        mutex_viewing_mode.lock().unwrap()
                            .send(Commands { viewing_mode: Some(live_mode_settings), ..Default::default() })
                            .unwrap();
                    }

                    Ok(Response::with((status::Ok, serde_json::to_string(&ret).unwrap())))
                },
                // TODO: handle errors
                Ok(None) => {
                    error!("Error");
                    return Ok(Response::with(status::Ok));
                },
                Err(_) => {
                    error!("Error");
                    return Ok(Response::with(status::Ok));
                },

            }
        }, "viewing_mode");

        let mutex_garage_door = mutex_c.clone();
        router.post("/gdo/", move |_: &mut Request| {
            mutex_garage_door.lock().unwrap()
                .send(Commands { garage_door_opener: Some(()), ..Default::default() })
                .unwrap();

            Ok(Response::with((status::Ok, "")))
        }, "garage_door");

        let status_lock_main = status_lock.clone();
        router.get("/status", move |_: &mut Request| {
            let status = status_lock_main.read().unwrap();
            Ok(Response::with((status::Ok, serde_json::to_string(&(*status)).unwrap())))
        }, "status");

        const CSV_LINE_SIZE: i64 = 65;

        router.get("/status/history.csv", move |req: &mut Request| {
            let mut s = String::new();
            let query = req.get_ref::<UrlEncodedQuery>();
            let hours: i64 = query.map(|map| {
                map.get("hours")
                    .map(|h| h.get(0).unwrap().parse::<i64>().unwrap())
                    .unwrap_or(12)
            }).unwrap_or(12);

            let result = File::open("history.csv").and_then(|mut file| {
                file.seek(SeekFrom::End(-CSV_LINE_SIZE * hours * 60 * 2))?;
                file.read_to_string(&mut s)
            });
            match result  {
                Ok(_) => Ok(Response::with((status::Ok, GzipWriter(s.as_bytes())))),
                Err(err) => { error!("Error: {}", err); Ok(Response::with(status::Ok)) }
            }
        }, "history");

        let ra_status = status_lock.clone();
        ra_router.get("/", move |_: &mut Request| {
            let status = ra_status.read().unwrap();
            let response = "<RA><T1>".to_string() + &(status.currentTempF * 10.0).to_string() + &"</T1><PH>" + &(status.pH * 100.0).to_string() + "</PH></RA>";
            Ok(Response::with((status::Ok, response)))
        }, "reef_angel");

        let mut mount = Mount::new();
        mount.mount("/api", router);
        mount.mount("/sa", ra_router);
        mount.mount("/", Static::new(Path::new("static/")));

        let mut chain = Chain::new(mount);
        // chain.link_before(BasicAuth);
        chain.link_before(::persistent::Read::<bodyparser::MaxBodyLength>::one(MAX_BODY_LENGTH));

        Iron::new(chain).http("192.168.1.243:80").unwrap();
    }

    // #[derive(Debug)]
    // struct AuthError;
    // struct BasicAuth;
    
    // impl Error for AuthError {
    //     fn description(&self) -> &str { "authentication error" }
    // }

    // impl fmt::Display for AuthError {
    //     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    //         fmt::Display::fmt("authentication error", f)
    //     }
    // }

    // impl middleware::BeforeMiddleware for BasicAuth {
    //     fn before(&self, req: &mut Request) -> IronResult<()> {
    //         if let Some(host) = req.headers.get::<headers::Host>() {
    //             if host.hostname == "192.168.1.243" { return Ok(()); }
    //         }
    //         if req.url.path().get(0).unwrap().to_string() == "sa" {
    //             return Ok(());
    //         }
    //         match req.headers.get::<headers::Authorization<headers::Basic>>() {
    //             Some(&headers::Authorization(headers::Basic { ref username, password: Some(ref password) })) => {
    //                 if username == "jon" && password == "aquamon1!" {
    //                     Ok(())
    //                 } else {
    //                     Err(IronError {
    //                         error: Box::new(AuthError),
    //                         response: Response::with((status::Unauthorized, "Wrong username or password."))
    //                     })
    //                 }
    //             }
    //             Some(&headers::Authorization(headers::Basic { username: _, password: None })) => {
    //                 Err(IronError {
    //                     error: Box::new(AuthError),
    //                     response: Response::with((status::Unauthorized, "Missing password"))
    //                 })
    //             }
    //             None => {
    //                 let mut hs = headers::Headers::new();
    //                 hs.set_raw("WWW-Authenticate", vec![b"Basic realm=\"main\"".to_vec()]);
    //                 Err(IronError {
    //                     error: Box::new(AuthError),
    //                     response: Response {
    //                         status: Some(status::Unauthorized),
    //                         headers: hs,
    //                         extensions: TypeMap::new(),
    //                         body: None
    //                     }
    //                 })
    //             }        
    //         }
    //     }
    // }

}

#[test]
fn it_works() {
}
