//! Medicine schedules with history.

use crate::dates;
use crate::model::Med;

/// Is a medicine record due on `date`? from ≤ date ≤ until and the rule matches.
pub fn is_due(m: &Med, date: &str) -> bool {
    if m.from.is_empty() || dates::days_between(&m.from, date) < 0 {
        return false;
    }
    if let Some(u) = &m.until {
        if dates::days_between(date, u) < 0 {
            return false;
        }
    }
    match m.freq.as_str() {
        "weekly" => m.days.contains(&dates::weekday(date)),
        "monthly" => m.dom.is_some_and(|d| d as u32 == dates::day_of_month(date)),
        _ => true,
    }
}

/// Status of a dose: "taken", "missed" (past day), "overdue" (today, past its time) or "due".
pub fn status(m: &Med, date: &str, today: &str, now_hhmm: &str, taken: bool) -> &'static str {
    if taken {
        return "taken";
    }
    let d = dates::days_between(today, date);
    if d < 0 {
        return "missed";
    }
    if d == 0 {
        if let (Some(t), Some(n)) = (dates::hhmm_minutes(&m.time), dates::hhmm_minutes(now_hhmm)) {
            if n > t {
                return "overdue";
            }
        }
    }
    "due"
}

/// Do two records differ in their schedule (not just name/dose)?
pub fn schedule_changed(a: &Med, b: &Med) -> bool {
    a.freq != b.freq || a.days != b.days || a.dom != b.dom || a.time != b.time
}

/// Normalise and check an edited medicine.
pub fn validate(m: &mut Med) -> Result<(), String> {
    m.name = m.name.trim().chars().take(80).collect();
    m.dose = m.dose.trim().chars().take(40).collect();
    if m.name.is_empty() {
        return Err("Give the medicine a name.".into());
    }
    if !crate::model::FREQS.contains(&m.freq.as_str()) {
        m.freq = "daily".into();
    }
    m.days.retain(|d| *d < 7);
    m.days.sort_unstable();
    m.days.dedup();
    if m.freq == "weekly" && m.days.is_empty() {
        return Err("Pick at least one day.".into());
    }
    if m.freq == "monthly" && !m.dom.is_some_and(|d| (1..=31).contains(&d)) {
        return Err("Pick a day of the month (1–31).".into());
    }
    if m.freq != "weekly" {
        m.days.clear();
    }
    if m.freq != "monthly" {
        m.dom = None;
    }
    m.time = m.time.trim().to_string();
    if !m.time.is_empty() {
        match dates::hhmm_minutes(&m.time) {
            Some(x) => m.time = format!("{:02}:{:02}", x / 60, x % 60),
            None => return Err("Time must be HH:MM or empty.".into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn med(freq: &str) -> Med {
        Med { group_id: "g".into(), name: "Vit D".into(), freq: freq.into(), from: "2026-11-01".into(), active: true, ..Default::default() }
    }

    #[test]
    fn due_rules() {
        let m = med("daily");
        assert!(is_due(&m, "2026-11-01"));
        assert!(!is_due(&m, "2026-10-31"));
        let mut m2 = m.clone();
        m2.until = Some("2026-11-10".into());
        assert!(is_due(&m2, "2026-11-10"));
        assert!(!is_due(&m2, "2026-11-11"));
        let mut w = med("weekly");
        w.days = vec![1, 3]; // Mon, Wed
        assert!(is_due(&w, "2026-11-23")); // Monday
        assert!(!is_due(&w, "2026-11-22")); // Sunday
        let mut mo = med("monthly");
        mo.dom = Some(15);
        assert!(is_due(&mo, "2026-11-15"));
        assert!(!is_due(&mo, "2026-11-16"));
    }

    #[test]
    fn statuses() {
        let mut m = med("daily");
        m.time = "08:00".into();
        assert_eq!(status(&m, "2026-11-22", "2026-11-22", "07:59", false), "due");
        assert_eq!(status(&m, "2026-11-22", "2026-11-22", "08:01", false), "overdue");
        assert_eq!(status(&m, "2026-11-21", "2026-11-22", "07:00", false), "missed");
        assert_eq!(status(&m, "2026-11-21", "2026-11-22", "07:00", true), "taken");
        m.time = String::new();
        assert_eq!(status(&m, "2026-11-22", "2026-11-22", "23:00", false), "due");
    }

    #[test]
    fn validation() {
        let mut m = med("weekly");
        assert!(validate(&mut m).is_err());
        m.days = vec![5, 1, 1];
        m.time = "8:5".into();
        assert!(validate(&mut m).is_err());
        m.time = "8:05".into();
        validate(&mut m).unwrap();
        assert_eq!(m.days, vec![1, 5]);
        assert_eq!(m.time, "08:05");
        let mut m = med("monthly");
        m.dom = Some(40);
        assert!(validate(&mut m).is_err());
    }
}
