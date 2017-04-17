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

extern crate serde_json;

pub mod server {
    use persistent::Read;
    use iron::status;
    use iron::prelude::*;
    // use iron::{Iron, Request, Response, IronResult};

    use mount::Mount;
    use router::Router;
    use staticfile::Static;

    use std::path::Path;
    use std::vec::Vec;

    use std::sync::{Mutex, RwLock, Arc};
    use std::sync::mpsc::Sender;
     
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
    pub struct Config {
        pub lights: LightSettings,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct Status {
        pub currentTempF: f32,
        pub depth: u8,
        // pub pH: f32,
        // pub timestamp: String,
        // TODO: map of on/off triggers
        // TODO: water level
        //
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct TemperatureSettings {
        pub setPoint: f32,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct DepthSettingsMaintain {
        pub low: u8,
        pub high: u8,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct DepthSettingsDepthValues {
        pub low: u8,
        pub high: u8,
        pub highInches: f32,
        pub tankSurfaceArea: u16,
        pub pumpGph: f32,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
    pub struct DepthSettings {
        pub maintainRange: DepthSettingsMaintain,
        pub depthValues: DepthSettingsDepthValues,
    }

    // Used to send commands to the main service
    #[derive(Default)]
    pub struct Commands {
        pub temperature_settings: Option<TemperatureSettings>,
        pub depth_settings: Option<DepthSettings>,
        pub lighting_schedule: Option<LightingSchedule>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Settings {
        pub temperature_settings: TemperatureSettings,
        pub depth_settings: DepthSettings,
        pub lighting_schedule: LightingSchedule,
    }

    // impl fmt::Display for Config {
    //     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    //         write!(f, "(Max {}, Min {}, Fan {} hours)", self.maxTempF, self.minTempF, self.fanDurationHours)
    //     }
    // }

    pub fn start(tx_live: Sender<Config>, status_lock: Arc<RwLock<Status>>, tx_command: Sender<Commands>, settings: Settings) {
        let mut router = Router::new();
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
                            .send(Commands { temperature_settings: Some(temperature_settings), ..Default::default() })
                            .unwrap();
                    }
                     
                    { 
                        let mut c = writer_temp_settings.write().unwrap();
                        *c = temperature_settings.clone();
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

        router.post("/lighting/live", move |req: &mut Request| {
            let body = req.get::<bodyparser::Struct<Config>>();
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

        router.get("/status", move |_: &mut Request| {
            let status = status_lock.read().unwrap();
            Ok(Response::with((status::Ok, serde_json::to_string(&(*status)).unwrap())))
        }, "status");

        let mut mount = Mount::new();
        mount.mount("/api", router);
        mount.mount("/", Static::new(Path::new("static/")));

        let mut chain = Chain::new(mount);
        chain.link_before(Read::<bodyparser::MaxBodyLength>::one(MAX_BODY_LENGTH));

        Iron::new(chain).http("192.168.1.243:80").unwrap();
    }
}

#[test]
fn it_works() {
}
