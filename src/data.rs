use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::{Date, Time, Weekday};

#[derive(Serialize, Deserialize, Debug)]
pub struct AppData {
    pub meta: Meta,
    pub calendar: Calendar,
    pub schedules: ScheduleStore,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    pub name: String,
    pub periods: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Calendar {
    pub default: Week,
    pub special: Vec<SpecialDay>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Week {
    pub mon: Option<String>,
    pub tue: Option<String>,
    pub wed: Option<String>,
    pub thu: Option<String>,
    pub fri: Option<String>,
    pub sat: Option<String>,
    pub sun: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpecialDay {
    pub on: Date,
    pub until: Option<Date>,
    pub schedule: String,
    pub comment: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleStore {
    pub schedules: HashMap<String, Schedule>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Schedule {
    pub comment: Option<String>,
    pub periods: Vec<Period>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Period {
    pub msg: String,
    pub start: Time,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CurrentSection {
    pub schedule_name: String,
    pub schedule_comment: Option<String>,
    pub current_period: Period,
    pub next_period: Option<Period>,
    pub current_period_end: Option<Time>,
    pub meta_name: String,
    pub meta_periods: Vec<String>,
}

pub fn load_app_data() -> AppData {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/data.postcard"));
    postcard::from_bytes(bytes).expect("Failed to deserialize data.postcard")
}

impl AppData {
    pub fn schedule_name_for_date(&self, date: Date) -> Option<&str> {
        for special in &self.calendar.special {
            if is_special_day_match(date, special) {
                return Some(special.schedule.as_str());
            }
        }
        let week = &self.calendar.default;
        match date.weekday() {
            Weekday::Monday => week.mon.as_deref(),
            Weekday::Tuesday => week.tue.as_deref(),
            Weekday::Wednesday => week.wed.as_deref(),
            Weekday::Thursday => week.thu.as_deref(),
            Weekday::Friday => week.fri.as_deref(),
            Weekday::Saturday => week.sat.as_deref(),
            Weekday::Sunday => week.sun.as_deref(),
        }
    }

    pub fn current_section(&self, date: Date, time: Time) -> Option<CurrentSection> {
        let schedule_name = self.schedule_name_for_date(date)?;
        let schedule = self.schedules.schedules.get(schedule_name)?;
        let mut current_index = None;
        for (idx, period) in schedule.periods.iter().enumerate() {
            if time >= period.start {
                current_index = Some(idx);
            } else {
                break;
            }
        }
        let current_index = current_index?;
        let current_period = schedule.periods.get(current_index)?.clone();
        let next_period = schedule.periods.get(current_index + 1).cloned();
        let current_period_end = next_period.as_ref().map(|next| next.start);
        Some(CurrentSection {
            schedule_name: schedule_name.to_string(),
            schedule_comment: schedule.comment.clone(),
            current_period,
            next_period,
            current_period_end,
            meta_name: self.meta.name.clone(),
            meta_periods: self.meta.periods.clone(),
        })
    }
}

fn is_special_day_match(date: Date, special: &SpecialDay) -> bool {
    match special.until {
        Some(until) => date >= special.on && date <= until,
        None => date == special.on,
    }
}
