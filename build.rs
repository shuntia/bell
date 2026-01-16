use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use time::{Date, Time, macros::format_description};

fn main() {
    let selected_schedule = option_env!("SELECTED_SCHEDULE").unwrap_or("lahs");
    let schedule_dir = option_env!("SCHEDULE_DIR").unwrap_or("schedules");
    let schedule = PathBuf::from(format!("{}/{}", schedule_dir, selected_schedule));
    if !schedule.exists() {
        panic!(
            "Selected schedule '{}' does not exist in directory '{}'",
            selected_schedule, schedule_dir
        );
    }
    let meta = read_meta(&schedule.join("meta.json"));
    let calendar = read_calendar(&schedule.join("calendar.bell"));
    let schedules = read_schedules(&schedule.join("schedules.bell"));
    verify_schedules(&schedules, &calendar);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let data_out = out_dir.join("data.postcard");
    let data = AppData {
        meta,
        calendar,
        schedules,
    };
    let data_bytes = postcard::to_stdvec(&data).expect("Failed to serialize data");
    std::fs::write(data_out, data_bytes).expect("Failed to write data.postcard");
}

fn verify_schedules(schedules: &ScheduleStore, calendar: &Calendar) {
    let mut calendar_schedules = HashSet::new();
    let week = &calendar.default;
    if let Some(name) = &week.sun {
        calendar_schedules.insert(name.clone());
    }
    if let Some(name) = &week.mon {
        calendar_schedules.insert(name.clone());
    }
    if let Some(name) = &week.tue {
        calendar_schedules.insert(name.clone());
    }
    if let Some(name) = &week.wed {
        calendar_schedules.insert(name.clone());
    }
    if let Some(name) = &week.thu {
        calendar_schedules.insert(name.clone());
    }
    if let Some(name) = &week.fri {
        calendar_schedules.insert(name.clone());
    }
    if let Some(name) = &week.sat {
        calendar_schedules.insert(name.clone());
    }
    for special in &calendar.special {
        calendar_schedules.insert(special.schedule.clone());
    }
    for name in schedules.schedules.keys() {
        if !calendar_schedules.contains(name) {
            panic!("Schedule '{}' is not referenced in calendar", name);
        }
    }
}

fn read_meta(meta_path: &Path) -> Meta {
    let meta_data = std::fs::read_to_string(meta_path).expect("Failed to read meta.json");
    let meta: Meta = serde_json::from_str(&meta_data).expect("Failed to parse meta.json");
    meta
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    pub name: String,
    pub periods: Vec<String>,
}

fn read_calendar(calendar: &Path) -> Calendar {
    let date_format = format_description!("[month]/[day]/[year]");
    let mut file = File::open(calendar).unwrap();
    let mut buf = String::with_capacity(file.metadata().unwrap().len() as usize);
    file.read_to_string(&mut buf).unwrap();
    let mut iter = buf.lines().map(|el| el.trim());
    if iter.next().unwrap() != "* Default Week" {
        panic!("Invalid start of calendar file");
    }
    let mut default_week = Week::default();
    for next in iter.by_ref() {
        if next.is_empty() {
            continue;
        }
        if next == "* Special Days" {
            break;
        }
        let mut parts = next.split_whitespace();
        let day = parts.next().unwrap_or("").trim();
        let schedule = parts.next().unwrap_or("").trim();
        if schedule.is_empty() {
            panic!("Missing schedule for default week");
        }
        match day {
            "Sun" => default_week.sun = Some(schedule.to_string()),
            "Mon" => default_week.mon = Some(schedule.to_string()),
            "Tue" => default_week.tue = Some(schedule.to_string()),
            "Wed" => default_week.wed = Some(schedule.to_string()),
            "Thu" => default_week.thu = Some(schedule.to_string()),
            "Fri" => default_week.fri = Some(schedule.to_string()),
            "Sat" => default_week.sat = Some(schedule.to_string()),
            _ => panic!("Invalid day in default week"),
        }
    }
    let mut special_days = Vec::new();
    for next in iter {
        if next.is_empty() {
            continue;
        }
        let (before_comment, comment) = match next.split_once('#') {
            Some((left, right)) => (left.trim(), Some(right.trim().to_string())),
            None => (next.trim(), None),
        };
        let mut parts = before_comment.split_whitespace();
        let date_str = parts.next().unwrap_or("").trim();
        let schedule = parts.next().unwrap_or("").trim();
        if date_str.is_empty() || schedule.is_empty() {
            panic!("Invalid special day entry");
        }
        let (on, until) = if date_str.contains('-') {
            let mut dates = date_str.splitn(2, '-');
            let on = Date::parse(dates.next().unwrap(), date_format).unwrap();
            let until = Date::parse(dates.next().unwrap(), date_format).unwrap();
            (on, Some(until))
        } else {
            let on = Date::parse(date_str, date_format).unwrap();
            (on, None)
        };
        special_days.push(SpecialDay {
            on,
            until,
            schedule: schedule.to_string(),
            comment,
        });
    }
    Calendar {
        default: default_week,
        special: special_days,
    }
}

fn read_schedules(schedules_path: &Path) -> ScheduleStore {
    let mut file = File::open(schedules_path).unwrap();
    let mut buf = String::with_capacity(file.metadata().unwrap().len() as usize);
    file.read_to_string(&mut buf).unwrap();
    let iter = buf.lines().map(|el| el.trim());
    let mut schedules: HashMap<String, Schedule> = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current_comment: Option<String> = None;
    let mut current_periods: Vec<Period> = Vec::new();

    for next in iter {
        if next.is_empty() {
            continue;
        }
        if let Some(header) = next.strip_prefix('*') {
            if let Some(name) = current_name.take() {
                let schedule = Schedule {
                    comment: current_comment.take(),
                    periods: std::mem::take(&mut current_periods),
                };
                if schedules.insert(name, schedule).is_some() {
                    panic!("Duplicate schedule name in schedules.bell");
                }
            }
            let header = header.trim();
            let (before_comment, comment) = match header.split_once('#') {
                Some((left, right)) => (left.trim(), Some(right.trim().to_string())),
                None => (header, None),
            };
            let mut parts = before_comment.split_whitespace();
            let name = parts.next().unwrap_or("").trim();
            if name.is_empty() {
                panic!("Missing schedule name in schedules.bell");
            }
            current_name = Some(name.to_string());
            current_comment = comment.filter(|val| !val.is_empty());
            continue;
        }
        if current_name.is_none() {
            panic!("Schedule period found before any schedule header");
        }
        let (start, msg) = split_start_message(next);
        let start = parse_start_time(start);
        current_periods.push(Period {
            start,
            msg: msg.to_string(),
        });
    }
    if let Some(name) = current_name.take() {
        let schedule = Schedule {
            comment: current_comment.take(),
            periods: std::mem::take(&mut current_periods),
        };
        if schedules.insert(name, schedule).is_some() {
            panic!("Duplicate schedule name in schedules.bell");
        }
    }

    ScheduleStore { schedules }
}

fn split_start_message(line: &str) -> (&str, &str) {
    if let Some((idx, _)) = line.char_indices().find(|(_, ch)| ch.is_whitespace()) {
        let (start, rest) = line.split_at(idx);
        let start = start.trim();
        let msg = rest.trim();
        if start.is_empty() || msg.is_empty() {
            panic!("Invalid period entry in schedules.bell");
        }
        (start, msg)
    } else {
        panic!("Invalid period entry in schedules.bell");
    }
}

fn parse_start_time(raw: &str) -> Time {
    let (hour_str, minute_str) = raw
        .split_once(':')
        .expect("Invalid time entry in schedules.bell");
    let hour: u8 = hour_str.trim().parse().expect("Invalid hour in schedules.bell");
    let minute: u8 = minute_str.trim().parse().expect("Invalid minute in schedules.bell");
    Time::from_hms(hour, minute, 0).expect("Invalid time in schedules.bell")
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Calendar {
    pub default: Week,
    pub special: Vec<SpecialDay>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Week {
    mon: Option<String>,
    tue: Option<String>,
    wed: Option<String>,
    thu: Option<String>,
    fri: Option<String>,
    sat: Option<String>,
    sun: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpecialDay {
    on: Date,
    until: Option<Date>,
    schedule: String,
    comment: Option<String>,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Period {
    pub msg: String,
    pub start: Time,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppData {
    pub meta: Meta,
    pub calendar: Calendar,
    pub schedules: ScheduleStore,
}
