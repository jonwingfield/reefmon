use chrono::{NaiveTime};

#[derive(Debug)]
pub struct ScheduleLeg {
    pub intensity: u8,
    pub intensities: [u8; 6],
    pub start_time: NaiveTime,
}

#[derive(Debug)]
pub struct Schedule {
    legs: Vec<ScheduleLeg>,
}

impl ScheduleLeg {
    pub fn weighted_intensity(&self, index: usize) -> u8 {
        let intensity = self.intensity as f32 / 255.0;

        (self.intensities[index] as f32 * intensity).round() as u8
    }

    pub fn weighted_intensities(&self) -> [u8; 6] {
        let mut intensities = [0_u8; 6];
         
        for i in 0..6 {
            intensities[i] = self.weighted_intensity(i);
        }

        intensities
    }
}

impl Schedule {
    pub fn new(legs: Vec<ScheduleLeg>) -> Schedule {
        Schedule { legs: legs }
    }

    pub fn default() -> Schedule {
        Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [0_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [0_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ])
    }

    pub fn get_intensities(&self, time: NaiveTime) -> [u8; 6] {
        if time <= self.legs[0].start_time || time >= self.legs.last().unwrap().start_time {
            return [0_u8; 6];
        }

        let next_pos = self.legs.iter().position(|leg| {
            leg.start_time > time
        }).unwrap_or(0);

        let active_pos = next_pos-1;
        let active = (&self.legs[active_pos], &self.legs[next_pos]);

        trace!("Active: {:?} {:?} {:?} {:?} {:?}", active.0.start_time, active.1.start_time, active_pos, next_pos, time);
        
        Schedule::calc_intensities(time, active.0, active.1)
    }

    fn calc_intensities(current_time: NaiveTime, a: &ScheduleLeg, b: &ScheduleLeg) -> [u8; 6] {
        let minutes_in_interval = b.start_time.signed_duration_since(a.start_time).num_minutes() as f32;
        let elapsed_minutes = current_time.signed_duration_since(a.start_time).num_minutes() as f32;

        let mut intensities = [0_u8; 6];
         
        for i in 0..6 {
            let ai = a.weighted_intensity(i) as f32;
            let bi = b.weighted_intensity(i) as f32;

            let interpolated = ai + (bi - ai) / minutes_in_interval.max(1.0) * elapsed_minutes;
            assert!(interpolated >= 0.0);
            intensities[i] = interpolated.round() as u8;
        }

        intensities
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test] 
    fn intensities_zero_when_time_before_first_leg() {
        let schedule = Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 100, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(11, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ]);
        let intensities = schedule.get_intensities(NaiveTime::from_hms(9, 0, 0));
        assert_eq!(intensities, [0_u8; 6]);
    }

    #[test] 
    fn intensities_zero_when_time_after_last_leg() {
        let schedule = Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 100, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(11, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ]);
        let intensities = schedule.get_intensities(NaiveTime::from_hms(17, 0, 0));
        assert_eq!(intensities, [0_u8; 6]);
    }

    #[test]
    fn intensities_match_when_exact_time() {
        let schedule = Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 100, intensities: [20, 30, 40, 50, 60, 70], start_time: NaiveTime::from_hms(11, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ]);
        let intensities = schedule.get_intensities(NaiveTime::from_hms(11, 0, 0));
        let weighted_intensities = schedule.legs[1].weighted_intensities();
        assert_eq!(intensities, weighted_intensities);
    }

    #[test]
    fn weighted_intensity_is_overall_intensity_times_individual() {
        let leg = ScheduleLeg { intensity: 100, intensities: [20, 30, 40, 50, 60, 70], start_time: NaiveTime::from_hms(11, 0, 0) };
        let weight: f32= 100.0 / 255.0;
        assert_eq!(leg.weighted_intensities(), [(weight * 20.0).round() as u8, 12, 16, 20, 24, 27]);
    }

    #[test]
    fn intensities_interpolated_linearly_when_increasing() {
        let schedule = Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 255, intensities: [20, 30, 40, 50, 60, 70], start_time: NaiveTime::from_hms(11, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ]);
        let intensities = schedule.get_intensities(NaiveTime::from_hms(10, 0, 0));
        assert_eq!(intensities, [10, 15, 20, 25, 30, 35]);
    }

    #[test]
    fn intensities_interpolated_linearly_when_decreasing() {
        let schedule = Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 255, intensities: [20, 30, 40, 50, 60, 70], start_time: NaiveTime::from_hms(11, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ]);
        let intensities = schedule.get_intensities(NaiveTime::from_hms(14, 0, 0));
        assert_eq!(intensities, [10, 15, 20, 25, 30, 35]);
    }

    #[test]
    fn intensities_interpolated_linearly_with_overall_intensity_and_elapsed_time_accounted_for() {
        let schedule = Schedule::new(vec![
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(9, 0, 0) },
            ScheduleLeg { intensity: 90, intensities: [20, 30, 40, 50, 60, 70], start_time: NaiveTime::from_hms(11, 0, 0) },
            ScheduleLeg { intensity: 170, intensities: [51, 62, 73, 60, 44, 88], start_time: NaiveTime::from_hms(14, 0, 0) },
            ScheduleLeg { intensity: 0, intensities: [100_u8; 6], start_time: NaiveTime::from_hms(17, 0, 0) }
        ]);
        let intensities = schedule.get_intensities(NaiveTime::from_hms(13, 22, 0));
        assert_eq!(intensities, [28, 35, 42, 35, 27, 52]);

    }
}
