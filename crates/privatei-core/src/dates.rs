//! Local calendar dates as `YYYY-MM-DD` strings.
//!
//! "Today" is always the LOCAL calendar date. In Zurich, UTC is still
//! yesterday until 01:00 (02:00 in summer), so never derive it from UTC.

const DAY_MS: f64 = 86_400_000.0;

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil date for a day number.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

/// Validate and convert a civil date.
pub fn ymd_to_days(y: i64, m: u32, d: u32) -> Option<i64> {
    if !(1900..=2200).contains(&y) || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// Parse `YYYY-MM-DD` into a day number.
pub fn parse(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y = s.get(0..4)?.parse().ok()?;
    let m = s.get(5..7)?.parse().ok()?;
    let d = s.get(8..10)?.parse().ok()?;
    ymd_to_days(y, m, d)
}

pub fn is_date(s: &str) -> bool {
    parse(s).is_some()
}

pub fn format(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `date` shifted by `n` days. Invalid input is returned unchanged.
pub fn add_days(date: &str, n: i64) -> String {
    match parse(date) {
        Some(d) => format(d + n),
        None => date.to_string(),
    }
}

/// Whole days from `a` to `b` (b - a). 0 if either is invalid.
pub fn days_between(a: &str, b: &str) -> i64 {
    match (parse(a), parse(b)) {
        (Some(a), Some(b)) => b - a,
        _ => 0,
    }
}

/// Days between two instants, rounded so a 23 h or 25 h DST day counts as one.
pub fn days_between_ts(a_ms: i64, b_ms: i64) -> i64 {
    ((b_ms - a_ms) as f64 / DAY_MS).round() as i64
}

/// Day of week, Sunday = 0.
pub fn weekday(date: &str) -> u8 {
    parse(date).map(|d| (d + 4).rem_euclid(7) as u8).unwrap_or(0)
}

/// Day of month (1-31).
pub fn day_of_month(date: &str) -> u32 {
    parse(date).map(|d| civil_from_days(d).2).unwrap_or(0)
}

/// The local calendar date of an instant, given the local UTC offset in minutes.
pub fn local_date_at(ts_ms: i64, offset_min: i32) -> String {
    let local_ms = ts_ms + offset_min as i64 * 60_000;
    format(local_ms.div_euclid(86_400_000))
}

/// Local `HH:MM` of an instant, given the UTC offset in minutes.
pub fn local_hhmm_at(ts_ms: i64, offset_min: i32) -> String {
    let local_ms = ts_ms + offset_min as i64 * 60_000;
    let mins = local_ms.rem_euclid(86_400_000) / 60_000;
    format!("{:02}:{:02}", mins / 60, mins % 60)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// The system's current UTC offset in minutes (DST aware).
pub fn local_offset_min(ts_ms: i64) -> i32 {
    use chrono::{Local, Offset, TimeZone};
    match Local.timestamp_millis_opt(ts_ms) {
        chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => dt.offset().fix().local_minus_utc() / 60,
        chrono::LocalResult::None => 0,
    }
}

/// Today's LOCAL date.
pub fn today() -> String {
    let now = now_ms();
    local_date_at(now, local_offset_min(now))
}

/// The local time now as `HH:MM`.
pub fn now_hhmm() -> String {
    let now = now_ms();
    local_hhmm_at(now, local_offset_min(now))
}

/// Minutes since midnight for `HH:MM`, if valid.
pub fn hhmm_minutes(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 {
        return None;
    }
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for d in [-1000, 0, 1, 19_000, 20_500, 60_000] {
            assert_eq!(parse(&format(d)), Some(d));
        }
        assert_eq!(format(0), "1970-01-01");
        assert_eq!(add_days("2024-02-28", 1), "2024-02-29");
        assert_eq!(add_days("2023-02-28", 1), "2023-03-01");
        assert_eq!(add_days("2024-12-31", 1), "2025-01-01");
        assert!(parse("2023-02-29").is_none());
        assert!(parse("2023-13-01").is_none());
        assert!(parse("23-01-01").is_none());
    }

    #[test]
    fn weekdays() {
        assert_eq!(weekday("1970-01-01"), 4); // Thursday
        assert_eq!(weekday("2026-11-22"), 0); // Sunday
        assert_eq!(weekday("2026-11-23"), 1);
    }

    #[test]
    fn midnight_in_zurich() {
        // 2026-11-22 23:30 UTC is already 2026-11-23 00:30 in Zurich (UTC+1).
        let ts = days_from_civil(2026, 11, 22) * 86_400_000 + (23 * 60 + 30) * 60_000;
        assert_eq!(local_date_at(ts, 0), "2026-11-22");
        assert_eq!(local_date_at(ts, 60), "2026-11-23");
        assert_eq!(local_hhmm_at(ts, 60), "00:30");
        // Summer (UTC+2): 22:30 UTC on 2026-07-01 is 00:30 on 2026-07-02.
        let ts = days_from_civil(2026, 7, 1) * 86_400_000 + (22 * 60 + 30) * 60_000;
        assert_eq!(local_date_at(ts, 120), "2026-07-02");
        // One minute before local midnight stays on the day.
        let ts = days_from_civil(2026, 11, 22) * 86_400_000 + (22 * 60 + 59) * 60_000;
        assert_eq!(local_date_at(ts, 60), "2026-11-22");
    }

    #[test]
    fn dst_days_round() {
        // Zurich spring forward: local midnight 29 Mar (UTC+1) to 30 Mar (UTC+2) is 23 h.
        let a = days_from_civil(2026, 3, 29) * 86_400_000 - 60 * 60_000;
        let b = days_from_civil(2026, 3, 30) * 86_400_000 - 120 * 60_000;
        assert_eq!(b - a, 23 * 3_600_000);
        assert_eq!(days_between_ts(a, b), 1);
        // Fall back: 25 h day.
        let a = days_from_civil(2026, 10, 25) * 86_400_000 - 120 * 60_000;
        let b = days_from_civil(2026, 10, 26) * 86_400_000 - 60 * 60_000;
        assert_eq!(days_between_ts(a, b), 1);
        assert_eq!(days_between("2026-03-28", "2026-04-02"), 5);
    }

    #[test]
    fn hhmm() {
        assert_eq!(hhmm_minutes("08:30"), Some(510));
        assert_eq!(hhmm_minutes("8:30"), Some(510));
        assert_eq!(hhmm_minutes("24:00"), None);
        assert_eq!(hhmm_minutes(""), None);
    }
}
