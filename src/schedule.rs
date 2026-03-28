use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Weekday};
use crate::launchd::CalendarInterval;

pub fn next_run_time(
    calendar_intervals: &Option<Vec<CalendarInterval>>,
    start_interval: &Option<u64>,
    now: DateTime<Local>,
) -> Option<DateTime<Local>> {
    if let Some(intervals) = calendar_intervals {
        let candidates: Vec<DateTime<Local>> = intervals
            .iter()
            .filter_map(|ci| next_calendar_run(ci, now))
            .collect();
        return candidates.into_iter().min();
    }

    if let Some(secs) = start_interval {
        return Some(now + Duration::seconds(*secs as i64));
    }

    None
}

fn next_calendar_run(ci: &CalendarInterval, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let hour = ci.hour.unwrap_or(0) as u32;
    let minute = ci.minute.unwrap_or(0) as u32;

    if let Some(weekday) = ci.weekday {
        let target_weekday = match weekday {
            0 => Weekday::Sun,
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            6 => Weekday::Sat,
            _ => return None,
        };

        let mut candidate = now.date_naive();
        for _ in 0..7 {
            if candidate.weekday() == target_weekday {
                let time = NaiveTime::from_hms_opt(hour, minute, 0)?;
                let dt = candidate.and_time(time);
                let local = dt.and_local_timezone(Local).single()?;
                if local > now {
                    return Some(local);
                }
            }
            candidate += Duration::days(1);
        }
        return None;
    }

    let today_time = NaiveTime::from_hms_opt(hour, minute, 0)?;
    let today_dt = now.date_naive().and_time(today_time);
    let today_local = today_dt.and_local_timezone(Local).single()?;

    if today_local > now {
        Some(today_local)
    } else {
        let tomorrow = now.date_naive() + Duration::days(1);
        let dt = tomorrow.and_time(today_time);
        dt.and_local_timezone(Local).single()
    }
}

pub fn format_next_run(next: Option<DateTime<Local>>, now: DateTime<Local>) -> String {
    let Some(next) = next else {
        return "-".to_string();
    };

    let today = now.date_naive();
    let next_date = next.date_naive();

    if next_date == today {
        format!("{}", next.format("%H:%M"))
    } else if next_date == today + Duration::days(1) {
        format!("明日 {}", next.format("%H:%M"))
    } else {
        format!("{}", next.format("%m/%d %H:%M"))
    }
}
