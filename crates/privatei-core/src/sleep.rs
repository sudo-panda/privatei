//! Sleep: when you went to bed and when you woke up. A night is stored under the date
//! you woke up, so "last night" belongs to today.

use serde::Serialize;

/// "23:30" → minutes after midnight. None unless it's a real HH:MM.
pub fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

pub fn fmt_hhmm(m: u32) -> String {
    format!("{:02}:{:02}", (m / 60) % 24, m % 60)
}

/// Minutes asleep from bed to wake, across midnight if needed. None for 0 or over 20 hours.
pub fn duration(bed: &str, wake: &str) -> Option<f64> {
    let (b, w) = (parse_hhmm(bed)?, parse_hhmm(wake)?);
    let d = if w > b { w - b } else { w + 1440 - b };
    (d > 0 && d <= 20 * 60).then_some(d as f64)
}

/// Average clock time. Times are first moved onto one 24-hour stretch that starts at
/// `pivot` (minutes after midnight), so 23:30 and 00:30 average to midnight, not noon.
pub fn avg_clock(times: &[u32], pivot: u32) -> Option<String> {
    if times.is_empty() {
        return None;
    }
    let sum: u32 = times.iter().map(|t| if *t < pivot { t + 1440 } else { *t }).sum();
    Some(fmt_hhmm((sum as f64 / times.len() as f64).round() as u32 % 1440))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SleepView {
    pub bed: String,
    pub wake: String,
    pub mins: f64,
}

pub fn view(s: &crate::model::Sleep) -> Option<SleepView> {
    Some(SleepView { bed: s.bed.clone(), wake: s.wake.clone(), mins: duration(&s.bed, &s.wake)? })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_durations() {
        assert_eq!(parse_hhmm("7:05"), Some(425));
        assert_eq!(parse_hhmm("23:59"), Some(1439));
        for bad in ["24:00", "12:60", "12", "ab:cd", "", "1:5", "123:00"] {
            assert_eq!(parse_hhmm(bad), None, "{bad}");
        }
        assert_eq!(duration("23:30", "07:00"), Some(450.0), "across midnight");
        assert_eq!(duration("01:15", "08:45"), Some(450.0), "after midnight");
        assert_eq!(duration("14:00", "15:30"), Some(90.0), "a nap");
        assert_eq!(duration("07:00", "07:00"), None);
        assert_eq!(duration("08:00", "07:00"), None, "23 hours is a typo");
        assert_eq!(avg_clock(&[1410, 30], 720).as_deref(), Some("00:00"));
        assert_eq!(avg_clock(&[420, 480], 1080).as_deref(), Some("07:30"));
        assert_eq!(avg_clock(&[], 720), None);
    }
}
