//! Record types. Field names match the export file (camelCase).
//! Deserialisation is lenient so older files with numbers-as-strings still load.

use serde::{Deserialize, Serialize};

pub mod lenient {
    use serde::{Deserialize, Deserializer};
    use serde_json::Value;

    fn num(v: &Value) -> Option<f64> {
        match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().replace(',', ".").parse().ok(),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
        .filter(|x: &f64| x.is_finite())
    }

    pub fn f64<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
        Ok(num(&Value::deserialize(d)?).unwrap_or(0.0))
    }
    pub fn opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
        Ok(num(&Value::deserialize(d)?))
    }
    pub fn i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
        Ok(num(&Value::deserialize(d)?).unwrap_or(0.0) as i64)
    }
    pub fn u8_or_1<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
        Ok(match num(&Value::deserialize(d)?) {
            Some(x) if x >= 2.0 => 2,
            _ => 1,
        })
    }
    pub fn opt_u8<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
        Ok(num(&Value::deserialize(d)?).filter(|x| (1.0..=31.0).contains(x)).map(|x| x as u8))
    }
    pub fn bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Bool(b) => b,
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::String(s) => matches!(s.as_str(), "true" | "1" | "yes"),
            _ => false,
        })
    }
    pub fn bool_true<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::Null => true,
            Value::Bool(b) => b,
            Value::Number(n) => n.as_f64().unwrap_or(1.0) != 0.0,
            Value::String(s) => !matches!(s.as_str(), "false" | "0" | "no"),
            _ => true,
        })
    }
    pub fn string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => String::new(),
        })
    }
    pub fn opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::String(s) if !s.trim().is_empty() => Some(s),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
    }
    pub fn days<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let mut out: Vec<u8> = match Value::deserialize(d)? {
            Value::Array(a) => a.iter().filter_map(num).filter(|x| (0.0..7.0).contains(x)).map(|x| x as u8).collect(),
            _ => vec![],
        };
        out.sort_unstable();
        out.dedup();
        Ok(out)
    }
}

/// Vitamins and minerals tracked per food / per day.
/// Units: vitamin C mg, vitamin D µg, B12 µg, iron mg, calcium mg.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Micros {
    #[serde(deserialize_with = "lenient::f64")]
    pub vit_c: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub vit_d: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub b12: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub iron: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub calcium: f64,
}

/// Daily reference amounts for an adult man (vitamin C 90 mg, D 15 µg, B12 2.4 µg, iron 8 mg, calcium 1000 mg).
pub const MICRO_REF: Micros = Micros { vit_c: 90.0, vit_d: 15.0, b12: 2.4, iron: 8.0, calcium: 1000.0 };

/// (key, label, unit) in display order.
pub const MICRO_NAMES: [(&str, &str, &str); 5] = [
    ("vitC", "Vitamin C", "mg"),
    ("vitD", "Vitamin D", "µg"),
    ("b12", "Vitamin B12", "µg"),
    ("iron", "Iron", "mg"),
    ("calcium", "Calcium", "mg"),
];

impl Micros {
    pub fn add(&mut self, o: &Micros) {
        self.vit_c += o.vit_c;
        self.vit_d += o.vit_d;
        self.b12 += o.b12;
        self.iron += o.iron;
        self.calcium += o.calcium;
    }
    pub fn scale(&self, f: f64) -> Micros {
        Micros { vit_c: self.vit_c * f, vit_d: self.vit_d * f, b12: self.b12 * f, iron: self.iron * f, calcium: self.calcium * f }
    }
    pub fn values(&self) -> [f64; 5] {
        [self.vit_c, self.vit_d, self.b12, self.iron, self.calcium]
    }
    /// Clamp to sane ranges (AI answers, imports).
    pub fn clamped(&self) -> Micros {
        let c = |x: f64, max: f64| if x.is_finite() { x.clamp(0.0, max) } else { 0.0 };
        Micros {
            vit_c: c(self.vit_c, 2000.0),
            vit_d: c(self.vit_d, 250.0),
            b12: c(self.b12, 1000.0),
            iron: c(self.iron, 100.0),
            calcium: c(self.calcium, 5000.0),
        }
    }
    /// Round for storage/display.
    pub fn rounded(&self) -> Micros {
        let r = |x: f64| (x * 10.0).round() / 10.0;
        Micros { vit_c: r(self.vit_c), vit_d: r(self.vit_d), b12: r(self.b12), iron: r(self.iron), calcium: self.calcium.round() }
    }
}

fn yes() -> bool {
    true
}

pub const MEALS: [&str; 4] = ["Breakfast", "Lunch", "Snack", "Dinner"];
pub const STOCK_CATS: [&str; 5] = ["cooked", "chilled", "produce", "shelf", "freezer"];
pub const KINDS: [&str; 6] = ["gym", "run", "walk", "cycle", "swim", "other"];
pub const FREQS: [&str; 3] = ["daily", "weekly", "monthly"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Log {
    #[serde(deserialize_with = "lenient::string")]
    pub date: String,
    #[serde(deserialize_with = "lenient::string")]
    pub meal: String,
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::opt_f64")]
    pub grams: Option<f64>,
    #[serde(deserialize_with = "lenient::f64")]
    pub kcal: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub prot: f64,
    #[serde(deserialize_with = "lenient::bool")]
    pub est: bool,
    #[serde(deserialize_with = "lenient::i64")]
    pub ts: i64,
    /// Fiber in g; None when unknown (older rows, foods without data).
    #[serde(deserialize_with = "lenient::opt_f64", skip_serializing_if = "Option::is_none")]
    pub fiber: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub micros: Option<Micros>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PlanMeal {
    #[serde(deserialize_with = "lenient::string")]
    pub slot: String,
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::f64")]
    pub kcal: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub prot: f64,
    #[serde(deserialize_with = "lenient::bool")]
    pub done: bool,
    #[serde(deserialize_with = "lenient::opt_string", skip_serializing_if = "Option::is_none")]
    pub log_id: Option<String>,
    #[serde(deserialize_with = "lenient::opt_f64", skip_serializing_if = "Option::is_none")]
    pub fiber: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Plan {
    pub meals: Vec<PlanMeal>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Stock {
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::f64")]
    pub qty: f64,
    #[serde(deserialize_with = "lenient::string")]
    pub unit: String,
    #[serde(deserialize_with = "lenient::string")]
    pub cat: String,
    #[serde(deserialize_with = "lenient::string")]
    pub added: String,
    #[serde(deserialize_with = "lenient::opt_string", skip_serializing_if = "Option::is_none")]
    pub use_by: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ListItem {
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::string")]
    pub qty: String,
    #[serde(deserialize_with = "lenient::string")]
    pub cat: String,
    #[serde(deserialize_with = "lenient::opt_f64", skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(deserialize_with = "lenient::u8_or_1")]
    pub trip: u8,
    #[serde(deserialize_with = "lenient::bool")]
    pub checked: bool,
    #[serde(deserialize_with = "lenient::bool")]
    pub in_stock: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Med {
    #[serde(deserialize_with = "lenient::string")]
    pub group_id: String,
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::string")]
    pub dose: String,
    #[serde(deserialize_with = "lenient::string")]
    pub freq: String,
    #[serde(deserialize_with = "lenient::days")]
    pub days: Vec<u8>,
    #[serde(deserialize_with = "lenient::opt_u8", skip_serializing_if = "Option::is_none")]
    pub dom: Option<u8>,
    #[serde(deserialize_with = "lenient::string")]
    pub time: String,
    #[serde(deserialize_with = "lenient::string")]
    pub from: String,
    #[serde(deserialize_with = "lenient::opt_string", skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(default = "yes", deserialize_with = "lenient::bool_true")]
    pub active: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MedLog {
    #[serde(deserialize_with = "lenient::string")]
    pub date: String,
    #[serde(deserialize_with = "lenient::string")]
    pub med_id: String,
    #[serde(deserialize_with = "lenient::string")]
    pub at: String,
    #[serde(deserialize_with = "lenient::i64")]
    pub ts: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Workout {
    #[serde(deserialize_with = "lenient::string")]
    pub date: String,
    #[serde(deserialize_with = "lenient::string")]
    pub kind: String,
    #[serde(deserialize_with = "lenient::f64")]
    pub mins: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub kcal: f64,
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::string")]
    pub src: String,
    #[serde(deserialize_with = "lenient::i64")]
    pub ts: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Food {
    #[serde(deserialize_with = "lenient::string")]
    pub name: String,
    #[serde(deserialize_with = "lenient::f64")]
    pub kcal100: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub prot100: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub piece: f64,
    #[serde(deserialize_with = "lenient::opt_f64", skip_serializing_if = "Option::is_none")]
    pub fiber100: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub micros100: Option<Micros>,
    #[serde(deserialize_with = "lenient::string")]
    pub src: String,
    #[serde(deserialize_with = "lenient::i64")]
    pub ts: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Rules {
    #[serde(deserialize_with = "lenient::string")]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    #[serde(deserialize_with = "lenient::bool")]
    pub add_burn: bool,
    #[serde(deserialize_with = "lenient::f64")]
    pub weight_kg: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub kcal_target: f64,
    #[serde(deserialize_with = "lenient::f64")]
    pub prot_floor: f64,
    #[serde(default = "default_fiber", deserialize_with = "lenient::f64")]
    pub fiber_floor: f64,
}

fn default_fiber() -> f64 {
    30.0
}

impl Default for Settings {
    fn default() -> Self {
        Settings { add_burn: false, weight_kg: 82.0, kcal_target: 1500.0, prot_floor: 90.0, fiber_floor: 30.0 }
    }
}

impl Settings {
    /// Validate ranges: weight 30–300, kcal target 800–6000, protein floor 0–400.
    pub fn validate(&self) -> Result<(), String> {
        if !(30.0..=300.0).contains(&self.weight_kg) {
            return Err("Weight must be between 30 and 300 kg.".into());
        }
        if !(800.0..=6000.0).contains(&self.kcal_target) {
            return Err("Calorie target must be between 800 and 6000.".into());
        }
        if !(0.0..=400.0).contains(&self.prot_floor) {
            return Err("Protein floor must be between 0 and 400 g.".into());
        }
        if !(0.0..=100.0).contains(&self.fiber_floor) {
            return Err("Fiber floor must be between 0 and 100 g.".into());
        }
        Ok(())
    }
}

/// Device-local AI settings. Never synced or exported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub order: Vec<String>,
    #[serde(deserialize_with = "lenient::string")]
    pub gemini_key: String,
    #[serde(deserialize_with = "lenient::string")]
    pub gemini_model: String,
    #[serde(deserialize_with = "lenient::string")]
    pub claude_model: String,
    #[serde(deserialize_with = "lenient::string")]
    pub claude_path: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            order: vec!["claude".into(), "gemini".into()],
            gemini_key: String::new(),
            gemini_model: "gemini-flash-latest".into(),
            claude_model: String::new(),
            claude_path: String::new(),
        }
    }
}

/// Valid model names: `[A-Za-z0-9.-]{0,40}`.
pub fn valid_model_name(s: &str) -> bool {
    s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Meal slot guessed from the local time.
pub fn meal_for_time(hhmm: &str) -> &'static str {
    match crate::dates::hhmm_minutes(hhmm).unwrap_or(12 * 60) {
        m if m < 10 * 60 + 30 => "Breakfast",
        m if m < 14 * 60 + 30 => "Lunch",
        m if m < 17 * 60 + 30 => "Snack",
        _ => "Dinner",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meal_guess() {
        assert_eq!(meal_for_time("07:00"), "Breakfast");
        assert_eq!(meal_for_time("10:29"), "Breakfast");
        assert_eq!(meal_for_time("10:30"), "Lunch");
        assert_eq!(meal_for_time("14:29"), "Lunch");
        assert_eq!(meal_for_time("14:30"), "Snack");
        assert_eq!(meal_for_time("17:29"), "Snack");
        assert_eq!(meal_for_time("17:30"), "Dinner");
        assert_eq!(meal_for_time("23:59"), "Dinner");
    }

    #[test]
    fn lenient_parsing() {
        let l: Log = serde_json::from_str(r#"{"kcal":"120","prot":"4,5","est":1,"grams":null,"extra":true}"#).unwrap();
        assert_eq!(l.kcal, 120.0);
        assert_eq!(l.prot, 4.5);
        assert!(l.est);
        assert_eq!(l.grams, None);
        let m: Med = serde_json::from_str(r#"{"days":[3,1,1,9],"dom":"15"}"#).unwrap();
        assert_eq!(m.days, vec![1, 3]);
        assert_eq!(m.dom, Some(15));
        assert!(m.active);
        let i: ListItem = serde_json::from_str(r#"{"qty":4,"trip":"2"}"#).unwrap();
        assert_eq!(i.qty, "4");
        assert_eq!(i.trip, 2);
    }

    #[test]
    fn settings_ranges() {
        assert!(Settings::default().validate().is_ok());
        assert!(Settings { weight_kg: 29.0, ..Default::default() }.validate().is_err());
        assert!(Settings { kcal_target: 6001.0, ..Default::default() }.validate().is_err());
        assert!(Settings { prot_floor: -1.0, ..Default::default() }.validate().is_err());
        assert!(Settings { fiber_floor: 101.0, ..Default::default() }.validate().is_err());
        let old: Settings = serde_json::from_str(r#"{"weightKg":80,"kcalTarget":1600,"protFloor":100}"#).unwrap();
        assert_eq!(old.fiber_floor, 30.0, "older backups get the default fiber floor");
        assert!(Settings { prot_floor: 400.0, kcal_target: 800.0, weight_kg: 300.0, ..Default::default() }.validate().is_ok());
    }

    #[test]
    fn model_names() {
        assert!(valid_model_name(""));
        assert!(valid_model_name("claude-sonnet-4.5"));
        assert!(!valid_model_name("a b"));
        assert!(!valid_model_name("x;rm"));
        assert!(!valid_model_name(&"a".repeat(41)));
    }
}
