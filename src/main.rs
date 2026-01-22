use std::{
    io::{Write, stdout},
    thread::sleep,
    time::Duration,
};

use time::PrimitiveDateTime;

pub mod data;

fn main() {
    env_logger::init();
    let opts = match parse_args(std::env::args().skip(1)) {
        Ok(opts) => opts,
        Err(err) => {
            if err == "Requested help." {
                println!("{}", usage());
                std::process::exit(0);
            }
            eprintln!("{err}");
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };
    if let Err(err) = run(opts) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run(opts: Options) -> Result<(), String> {
    let data = data::load_app_data();
    if opts.once {
        if let Some((label, msg, remaining)) = current_or_next(&data) {
            let line = match &opts.format {
                OutputFormat::Plain => default_line(label, &msg, remaining),
                OutputFormat::Pattern(pattern) => {
                    format_line_with_pattern(pattern, label, &msg, remaining)
                }
            };
            print_line(line, true);
        } else {
            return Err("No current or upcoming periods found.".to_string());
        }
        return Ok(());
    }
    loop {
        sleep(Duration::from_secs(opts.interval_secs));
        if let Some((label, msg, remaining)) = current_or_next(&data) {
            let line = match &opts.format {
                OutputFormat::Plain => default_line(label, &msg, remaining),
                OutputFormat::Pattern(pattern) => {
                    format_line_with_pattern(pattern, label, &msg, remaining)
                }
            };
            print_line(line, false);
        }
    }
}

#[derive(Debug, Clone)]
enum OutputFormat {
    Plain,
    Pattern(String),
}

#[derive(Debug, Clone)]
struct Options {
    format: OutputFormat,
    once: bool,
    interval_secs: u64,
}

fn parse_args<I>(mut args: I) -> Result<Options, String>
where
    I: Iterator<Item = String>,
{
    let mut opts = Options {
        format: OutputFormat::Plain,
        once: false,
        interval_secs: 1,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--once" => opts.once = true,
            "--format" => {
                let value = args.next().ok_or_else(|| "Missing value for --format".to_string())?;
                if value == "plain" {
                    opts.format = OutputFormat::Plain;
                } else {
                    opts.format = OutputFormat::Pattern(value);
                }
            }
            "--interval" => {
                let value =
                    args.next().ok_or_else(|| "Missing value for --interval".to_string())?;
                opts.interval_secs = value
                    .parse()
                    .map_err(|_| "Invalid value for --interval".to_string())?;
            }
            "--help" | "-h" => return Err("Requested help.".to_string()),
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    Ok(opts)
}

fn usage() -> &'static str {
    "Usage: bell [--once] [--format plain|<pattern>] [--interval <secs>]
    --once                Print once and exit
    --format plain        Default output format (with label/message)
    --format <pattern>    Line pattern with tokens: [Label] [Period] [HH] [MM] [SS]
                          Example: \"Period: [Period] | [HH]:[MM]:[SS]\"
    --interval <secs>     Refresh interval for continuous mode (default: 1)"
}

fn current_or_next(data: &data::AppData) -> Option<(&'static str, String, time::Duration)> {
    let now_dt = time::OffsetDateTime::now_local().unwrap();
    let today = now_dt.date();
    let now = now_dt.time();
    let (label, msg, remaining) = match data.current_section(today, now) {
        Some(section) if section.current_period_end.is_some() => {
            let end = section.current_period_end.unwrap();
            let remaining = if end > now {
                end - now
            } else {
                time::Duration::ZERO
            };
            ("Current", section.current_period.msg, remaining)
        }
        _ => match next_period_from(data, now_dt) {
            Some((period, remaining)) => ("Next", period.msg, remaining),
            None => return None,
        },
    };
    Some((label, msg, remaining))
}

fn default_line(label: &str, msg: &str, remaining: time::Duration) -> String {
    format!(
        "{}: {} | Remaining: {}",
        label,
        msg,
        format_duration(remaining),
    )
}

fn print_line(line: String, newline: bool) {
    let mut out = stdout().lock();
    if newline {
        out.write_all(line.as_bytes()).unwrap();
        out.write_all(b"\n").unwrap();
    } else {
        let line = format!("\r{line}");
        out.write_all(line.as_bytes()).unwrap();
    }
    out.flush().unwrap();
}

fn format_duration(duration: time::Duration) -> String {
    format_duration_with_pattern(duration, "[HH]:[MM]:[SS]")
}

fn format_duration_with_pattern(duration: time::Duration, pattern: &str) -> String {
    let (hours, minutes, seconds) = duration_tokens(duration, pattern);
    pattern
        .replace("[HH]", &hours)
        .replace("[MM]", &minutes)
        .replace("[SS]", &seconds)
}

fn duration_tokens(duration: time::Duration, pattern: &str) -> (String, String, String) {
    let total_seconds = duration.whole_seconds().max(0);
    let mut hours = total_seconds / 3600;
    let mut minutes = (total_seconds % 3600) / 60;
    let mut seconds = total_seconds % 60;
    if !pattern.contains("[HH]") {
        minutes += hours * 60;
        hours = 0;
    }
    if !pattern.contains("[MM]") {
        seconds += minutes * 60;
        minutes = 0;
    }
    (
        hours.to_string(),
        format!("{:02}", minutes),
        format!("{:02}", seconds),
    )
}

fn format_line_with_pattern(
    pattern: &str,
    label: &str,
    period: &str,
    remaining: time::Duration,
) -> String {
    let (hours, minutes, seconds) = duration_tokens(remaining, pattern);
    pattern
        .replace("[Label]", label)
        .replace("[Period]", period)
        .replace("[HH]", &hours)
        .replace("[MM]", &minutes)
        .replace("[SS]", &seconds)
}

fn next_period_from(
    data: &data::AppData,
    now_dt: time::OffsetDateTime,
) -> Option<(data::Period, time::Duration)> {
    let offset = now_dt.offset();
    let mut date = now_dt.date();
    loop {
        let schedule_name = match data.schedule_name_for_date(date) {
            Some(name) => name,
            None => {
                date = date.next_day()?;
                continue;
            }
        };
        let schedule = data.schedules.schedules.get(schedule_name)?;
        let first = match schedule.periods.first() {
            Some(period) => period.clone(),
            None => {
                date = date.next_day()?;
                continue;
            }
        };
        if date == now_dt.date() && now_dt.time() >= first.start {
            date = date.next_day()?;
            continue;
        }
        let target = PrimitiveDateTime::new(date, first.start).assume_offset(offset);
        let remaining = target - now_dt;
        return Some((first, remaining));
    }
}
