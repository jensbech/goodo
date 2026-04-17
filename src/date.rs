use std::time::{SystemTime, UNIX_EPOCH};

pub fn today_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs / 86400
}

pub fn today() -> String {
    format_ymd(today_days())
}

pub fn format_ymd(days: i64) -> String {
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

pub fn days_to_ymd(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y as i32, m as u32, d as u32)
}

pub fn ymd_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m64 = m as u64;
    let month_part = if m64 > 2 { m64 - 3 } else { m64 + 9 };
    let doy = (153 * month_part + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

pub fn parse_date(input: &str) -> Option<String> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    if s == "today" {
        return Some(today());
    }
    if s == "tomorrow" {
        return Some(format_ymd(today_days() + 1));
    }
    if s == "yesterday" {
        return Some(format_ymd(today_days() - 1));
    }

    if let Some(rest) = s.strip_prefix('+') {
        if let Ok(n) = rest.parse::<i64>() {
            return Some(format_ymd(today_days() + n));
        }
    }
    if let Some(rest) = s.strip_prefix('-') {
        if let Ok(n) = rest.parse::<i64>() {
            return Some(format_ymd(today_days() - n));
        }
    }

    let day_index: Option<i64> = match s.as_str() {
        "sun" | "sunday" => Some(0),
        "mon" | "monday" => Some(1),
        "tue" | "tuesday" => Some(2),
        "wed" | "wednesday" => Some(3),
        "thu" | "thursday" => Some(4),
        "fri" | "friday" => Some(5),
        "sat" | "saturday" => Some(6),
        _ => None,
    };
    if let Some(target) = day_index {
        let today_dow = (today_days() + 4).rem_euclid(7);
        let mut delta = (target - today_dow).rem_euclid(7);
        if delta == 0 {
            delta = 7;
        }
        return Some(format_ymd(today_days() + delta));
    }

    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                return Some(format!("{:04}-{:02}-{:02}", y, m, d));
            }
        }
    }

    None
}

pub fn days_until(date: &str) -> Option<i64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y = parts[0].parse::<i32>().ok()?;
    let m = parts[1].parse::<u32>().ok()?;
    let d = parts[2].parse::<u32>().ok()?;
    Some(ymd_to_days(y, m, d) - today_days())
}

pub enum DueStatus {
    Overdue,
    Today,
    Soon,
    Neutral,
}

pub fn format_due(date: &str) -> (String, DueStatus) {
    let days = match days_until(date) {
        Some(d) => d,
        None => return (format!("[{date}]"), DueStatus::Neutral),
    };
    if days < 0 {
        (format!("[overdue {}d]", -days), DueStatus::Overdue)
    } else if days == 0 {
        ("[today]".to_string(), DueStatus::Today)
    } else if days == 1 {
        ("[tomorrow]".to_string(), DueStatus::Soon)
    } else if days < 7 {
        (format!("[in {days}d]"), DueStatus::Soon)
    } else {
        (format!("[{date}]"), DueStatus::Neutral)
    }
}
