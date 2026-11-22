//! Training burn and the workout paste import.

use crate::dates;
use serde::Serialize;

pub const MAX_SESSION_KCAL: f64 = 5000.0;

pub fn met(kind: &str) -> f64 {
    match kind {
        "gym" => 5.0,
        "run" => 9.8,
        "walk" => 3.5,
        "cycle" => 7.5,
        "swim" => 7.0,
        _ => 4.0,
    }
}

/// kcal = MET × 3.5 × weightKg / 200 × minutes, capped at 5000 per session.
pub fn burn(kind: &str, mins: f64, weight_kg: f64) -> f64 {
    (met(kind) * 3.5 * weight_kg / 200.0 * mins.max(0.0)).min(MAX_SESSION_KCAL).round()
}

/// The day's kcal target: kcalTarget, plus the day's burn when "add burn" is on.
pub fn day_target(kcal_target: f64, burn: f64, add_burn: bool) -> f64 {
    if add_burn {
        kcal_target + burn
    } else {
        kcal_target
    }
}

const STEMS: &[(&str, &[&str])] = &[
    ("run", &["run", "jog", "treadmill", "lauf"]),
    ("walk", &["walk", "step", "hike", "wander", "spazier"]),
    ("cycle", &["cycl", "bike", "spin", "rad", "velo", "fahrrad"]),
    ("swim", &["swim", "pool", "schwimm"]),
    ("gym", &["gym", "strength", "weight", "lift", "kraft", "hantel", "fitness", "crunch"]),
];

/// Sport from free text, matching stems at the START of a word.
pub fn sport(text: &str) -> Option<&'static str> {
    for w in text.to_lowercase().split(|c: char| !c.is_alphabetic()).filter(|w| !w.is_empty()) {
        for (kind, stems) in STEMS {
            if stems.iter().any(|s| w.starts_with(s)) {
                return Some(kind);
            }
        }
    }
    None
}

/// YYYY-MM-DD, DD.MM.YYYY or DD/MM/YYYY.
pub fn parse_date(s: &str) -> Option<String> {
    let s = s.trim();
    if dates::is_date(s) {
        return Some(s.to_string());
    }
    let sep = if s.contains('.') { '.' } else { '/' };
    let p: Vec<&str> = s.split(sep).collect();
    if p.len() != 3 || p[2].len() != 4 || p[0].is_empty() || p[0].len() > 2 || p[1].is_empty() || p[1].len() > 2 {
        return None;
    }
    let d: u32 = p[0].parse().ok()?;
    let m: u32 = p[1].parse().ok()?;
    let y: i64 = p[2].parse().ok()?;
    dates::ymd_to_days(y, m, d).map(dates::format)
}

/// Plain minutes ("45", "45 min"), H:MM:SS, or H:MM (1:05 = 65 minutes).
pub fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim().to_lowercase();
    let s = s.trim_end_matches("minutes").trim_end_matches("mins").trim_end_matches("min").trim_end_matches('m').trim();
    if s.contains(':') {
        let p: Vec<&str> = s.split(':').collect();
        let n: Vec<f64> = p.iter().map(|x| x.trim().parse::<f64>()).collect::<Result<_, _>>().ok()?;
        if n.iter().any(|x| *x < 0.0) {
            return None;
        }
        return match n.as_slice() {
            [h, m] if *m < 60.0 => Some(h * 60.0 + m),
            [h, m, sec] if *m < 60.0 && *sec < 60.0 => Some((h * 60.0 + m + sec / 60.0).round()),
            _ => None,
        };
    }
    s.replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0)
}

fn parse_kcal(s: &str) -> Option<f64> {
    let l = s.trim().to_lowercase();
    let n = l.trim_end_matches("kcal").trim_end_matches("cal").trim();
    if n.len() == l.len() {
        return None;
    }
    n.parse::<f64>().ok()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub date: String,
    pub kind: String,
    pub mins: f64,
    pub kcal: f64,
    pub name: String,
    pub kcal_estimated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteResult {
    pub sessions: Vec<Session>,
    pub skipped: usize,
}

/// Parse pasted workouts, one per line; fields separated by `,` `;` or tab.
pub fn parse_paste(text: &str, today: &str, weight_kg: f64) -> PasteResult {
    let mut sessions = vec![];
    let mut skipped = 0;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(line, today, weight_kg) {
            Some(s) => sessions.push(s),
            None => skipped += 1,
        }
    }
    PasteResult { sessions, skipped }
}

fn parse_line(line: &str, today: &str, weight_kg: f64) -> Option<Session> {
    let mut date = None;
    let mut mins = None;
    let mut kcal = None;
    let mut text = vec![];
    for f in line.split([',', ';', '\t']).map(str::trim).filter(|f| !f.is_empty()) {
        if date.is_none() {
            if let Some(d) = parse_date(f) {
                date = Some(d);
                continue;
            }
        }
        if let Some(k) = parse_kcal(f) {
            kcal = Some(k);
            continue;
        }
        if f.chars().any(|c| c.is_ascii_digit()) && parse_date(f).is_none() {
            if let Some(m) = parse_duration(f) {
                if mins.is_none() {
                    mins = Some(m);
                } else if kcal.is_none() {
                    kcal = Some(m);
                }
                continue;
            }
        }
        if f.chars().any(|c| c.is_alphabetic()) {
            text.push(f.to_string());
        }
    }
    let date = date?;
    let mins = mins?;
    if dates::days_between(today, &date) > 0 || mins <= 0.0 || mins > 1440.0 {
        return None;
    }
    let joined = text.join(" ");
    let kind = sport(&joined).unwrap_or("other").to_string();
    let (kcal, kcal_estimated) = match kcal.filter(|k| *k > 0.0) {
        Some(k) => (k.min(MAX_SESSION_KCAL).round(), false),
        None => (burn(&kind, mins, weight_kg), true),
    };
    let name: String = if joined.is_empty() { crate::food::capitalize(&kind) } else { joined.chars().take(80).collect() };
    Some(Session { date, kind, mins, kcal, name, kcal_estimated })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burn_formula() {
        // 5.0 × 3.5 × 82 / 200 × 60 = 430.5
        assert_eq!(burn("gym", 60.0, 82.0), 431.0);
        assert_eq!(burn("run", 30.0, 82.0), (9.8f64 * 3.5 * 82.0 / 200.0 * 30.0).round());
        assert_eq!(burn("walk", 60.0, 82.0), (3.5f64 * 3.5 * 82.0 / 200.0 * 60.0).round());
        assert_eq!(burn("yoga", 60.0, 82.0), (4.0f64 * 3.5 * 82.0 / 200.0 * 60.0).round());
        assert_eq!(burn("run", 100_000.0, 82.0), 5000.0, "capped");
        assert_eq!(day_target(1500.0, 400.0, true), 1900.0);
        assert_eq!(day_target(1500.0, 400.0, false), 1500.0);
    }

    #[test]
    fn sports() {
        assert_eq!(sport("Morning Run"), Some("run"));
        assert_eq!(sport("crunches"), Some("gym"), "crunches is gym, not run");
        assert_eq!(sport("Treadmill"), Some("run"));
        assert_eq!(sport("Radfahren"), Some("cycle"));
        assert_eq!(sport("Spaziergang"), Some("walk"));
        assert_eq!(sport("Schwimmen"), Some("swim"));
        assert_eq!(sport("Krafttraining"), Some("gym"));
        assert_eq!(sport("brunch"), None);
        assert_eq!(sport("yoga"), None);
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("45"), Some(45.0));
        assert_eq!(parse_duration("45 min"), Some(45.0));
        assert_eq!(parse_duration("1:05"), Some(65.0));
        assert_eq!(parse_duration("1:05:00"), Some(65.0));
        assert_eq!(parse_duration("0:30:30"), Some(31.0));
        assert_eq!(parse_duration("1:75"), None);
    }

    #[test]
    fn paste_import() {
        let text = "Date, Activity, Duration, Calories\n\
                    2026-11-20, Run, 30, 320\n\
                    21.11.2026; Crunches; 0:20\n\
                    22/11/2026\tSwim\t1:05:00\n\
                    2026-11-30, Run, 30\n\
                    2026-11-19, Walk, 2000\n\
                    hello world\n\
                    2026-11-18, cycling, 45 min, 400 kcal\n";
        let r = parse_paste(text, "2026-11-22", 82.0);
        assert_eq!(r.sessions.len(), 4);
        assert_eq!(r.skipped, 4, "header, future, >1440 minutes, junk");
        let s = &r.sessions[0];
        assert_eq!((s.date.as_str(), s.kind.as_str(), s.mins, s.kcal, s.kcal_estimated), ("2026-11-20", "run", 30.0, 320.0, false));
        let s = &r.sessions[1];
        assert_eq!((s.date.as_str(), s.kind.as_str(), s.mins), ("2026-11-21", "gym", 20.0));
        assert!(s.kcal_estimated);
        assert_eq!(s.kcal, burn("gym", 20.0, 82.0));
        let s = &r.sessions[2];
        assert_eq!((s.date.as_str(), s.kind.as_str(), s.mins), ("2026-11-22", "swim", 65.0));
        let s = &r.sessions[3];
        assert_eq!((s.kind.as_str(), s.mins, s.kcal), ("cycle", 45.0, 400.0));
    }

    #[test]
    fn date_never_minutes() {
        // A line with only a date and a sport has no duration: skipped, not "2026 minutes".
        let r = parse_paste("2026-11-20, Run\n20.11.2026 Run", "2026-11-22", 82.0);
        assert_eq!(r.sessions.len(), 0);
        assert_eq!(r.skipped, 2);
        assert_eq!(parse_date("31.02.2026"), None);
        assert_eq!(parse_date("05/11/2026").as_deref(), Some("2026-11-05"));
    }
}
