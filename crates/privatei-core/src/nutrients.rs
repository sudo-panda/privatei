//! The nutrient catalog: every nutrient Private-I tracks beyond kcal and protein.
//!
//! Amounts live in a map keyed by these ids (`Nut`). A missing key means "unknown",
//! never zero. References are daily amounts for an adult man (US FDA Daily Values /
//! DRIs; sodium, added sugar and saturated fat are upper limits).

use std::collections::BTreeMap;

/// Nutrient amounts by catalog key. Missing key = unknown.
pub type Nut = BTreeMap<String, f64>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// Aim to reach the reference.
    Target,
    /// Stay under the reference.
    Limit,
    /// Shown, no reference.
    Info,
}

#[derive(Debug, Clone, Copy)]
pub struct Def {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub group: &'static str,
    pub reference: f64,
    pub kind: Kind,
    /// Upper bound when cleaning untrusted values (per day or per item).
    pub max: f64,
    /// USDA FoodData Central nutrient ids, in order of preference.
    pub usda: &'static [u32],
    /// Column header in the Swiss Food Composition Database.
    pub swiss: &'static str,
}

use Kind::*;

macro_rules! def {
    ($key:literal, $label:literal, $unit:literal, $group:literal, $ref:expr, $kind:expr, $max:expr, [$($u:expr),*], $swiss:literal) => {
        Def { key: $key, label: $label, unit: $unit, group: $group, reference: $ref, kind: $kind, max: $max, usda: &[$($u),*], swiss: $swiss }
    };
}

pub const CATALOG: &[Def] = &[
    // Carbohydrates & fat
    def!("fiber", "Fiber", "g", "Carbs", 30.0, Target, 300.0, [1079], "Dietary fibres (g)"),
    def!("carbs", "Carbohydrates", "g", "Carbs", 275.0, Info, 2000.0, [1005, 1050], "Carbohydrates, available (g)"),
    def!("sugar", "Sugars", "g", "Carbs", 0.0, Info, 1000.0, [2000, 1063], "Sugars (g)"),
    def!("addedSugar", "Added sugar", "g", "Carbs", 50.0, Limit, 1000.0, [1235], ""),
    def!("starch", "Starch", "g", "Carbs", 0.0, Info, 1000.0, [1009], "Starch (g)"),
    def!("fat", "Fat", "g", "Fats", 78.0, Info, 1000.0, [1004, 1085], "Fat, total (g)"),
    def!("satFat", "Saturated fat", "g", "Fats", 17.0, Limit, 500.0, [1258], "Fatty acids, saturated (g)"),
    def!("monoFat", "Monounsaturated fat", "g", "Fats", 0.0, Info, 500.0, [1292], "Fatty acids, monounsaturated (g)"),
    def!("polyFat", "Polyunsaturated fat", "g", "Fats", 0.0, Info, 500.0, [1293], "Fatty acids, polyunsaturated (g)"),
    def!("transFat", "Trans fat", "g", "Fats", 2.0, Limit, 100.0, [1257], ""),
    def!("ala", "Omega-3 ALA", "g", "Fats", 1.6, Target, 100.0, [1404, 1270], "Alpha-linolenic acid (g)"),
    def!("epa", "Omega-3 EPA", "g", "Fats", 0.0, Info, 50.0, [1278], "Eicosapentaenoic acid EPA (g)"),
    def!("dha", "Omega-3 DHA", "g", "Fats", 0.0, Info, 50.0, [1272], "Docosahexaenoic acid (DHA) (g)"),
    def!("cholesterol", "Cholesterol", "mg", "Fats", 300.0, Limit, 5000.0, [1253], "Cholesterol (mg)"),
    // Vitamins
    def!("vitA", "Vitamin A", "µg", "Vitamins", 900.0, Target, 30000.0, [1106], "Vitamin A activity, RAE (µg)"),
    def!("vitB1", "Vitamin B1", "mg", "Vitamins", 1.2, Target, 500.0, [1165], "Vitamin B1 (thiamine) (mg)"),
    def!("vitB2", "Vitamin B2", "mg", "Vitamins", 1.3, Target, 500.0, [1166], "Vitamin B2 (riboflavin) (mg)"),
    def!("vitB3", "Vitamin B3", "mg", "Vitamins", 16.0, Target, 1000.0, [1167], "Niacin (mg)"),
    def!("vitB5", "Vitamin B5", "mg", "Vitamins", 5.0, Target, 1000.0, [1170], "Pantothenic acid (mg)"),
    def!("vitB6", "Vitamin B6", "mg", "Vitamins", 1.3, Target, 500.0, [1175], "Vitamin B6 (pyridoxine) (mg)"),
    def!("b12", "Vitamin B12", "µg", "Vitamins", 2.4, Target, 5000.0, [1178], "Vitamin B12 (cobalamin) (µg)"),
    def!("folate", "Folate", "µg", "Vitamins", 400.0, Target, 10000.0, [1190, 1177], "Folate (µg)"),
    def!("vitC", "Vitamin C", "mg", "Vitamins", 90.0, Target, 5000.0, [1162], "Vitamin C (ascorbic acid) (mg)"),
    def!("vitD", "Vitamin D", "µg", "Vitamins", 15.0, Target, 500.0, [1114], "Vitamin D (calciferol) (µg)"),
    def!("vitE", "Vitamin E", "mg", "Vitamins", 15.0, Target, 1000.0, [1109], "Vitamin E (α-tocopherol) (mg)"),
    def!("vitK", "Vitamin K", "µg", "Vitamins", 120.0, Target, 10000.0, [1185], ""),
    // Minerals
    def!("sodium", "Sodium", "mg", "Minerals", 2300.0, Limit, 50000.0, [1093], "Sodium (Na) (mg)"),
    def!("potassium", "Potassium", "mg", "Minerals", 3400.0, Target, 20000.0, [1092], "Potassium (K) (mg)"),
    def!("calcium", "Calcium", "mg", "Minerals", 1000.0, Target, 10000.0, [1087], "Calcium (Ca) (mg)"),
    def!("magnesium", "Magnesium", "mg", "Minerals", 420.0, Target, 5000.0, [1090], "Magnesium (Mg) (mg)"),
    def!("phosphorus", "Phosphorus", "mg", "Minerals", 700.0, Target, 10000.0, [1091], "Phosphorus (P) (mg)"),
    def!("iron", "Iron", "mg", "Minerals", 8.0, Target, 500.0, [1089], "Iron (Fe) (mg)"),
    def!("zinc", "Zinc", "mg", "Minerals", 11.0, Target, 500.0, [1095], "Zinc (Zn) (mg)"),
    def!("copper", "Copper", "mg", "Minerals", 0.9, Target, 100.0, [1098], ""),
    def!("manganese", "Manganese", "mg", "Minerals", 2.3, Target, 100.0, [1101], ""),
    def!("iodine", "Iodine", "µg", "Minerals", 150.0, Target, 10000.0, [1100], "Iodide (I) (µg)"),
    def!("selenium", "Selenium", "µg", "Minerals", 55.0, Target, 5000.0, [1103], "Selenium (Se) (µg)"),
    // Other
    def!("caffeine", "Caffeine", "mg", "Other", 400.0, Limit, 5000.0, [1057], ""),
    def!("alcohol", "Alcohol", "g", "Other", 0.0, Info, 1000.0, [1018], "Alcohol (g)"),
    def!("water", "Water", "g", "Other", 0.0, Info, 10000.0, [1051], "Water (g)"),
];

pub const GROUPS: [&str; 5] = ["Carbs", "Fats", "Vitamins", "Minerals", "Other"];

pub fn def(key: &str) -> Option<&'static Def> {
    CATALOG.iter().find(|d| d.key == key)
}

/// Round for storage: 3 significant-ish decimals for small values, 1 for large.
pub fn round(x: f64) -> f64 {
    if x.abs() >= 100.0 {
        x.round()
    } else if x.abs() >= 1.0 {
        (x * 10.0).round() / 10.0
    } else {
        (x * 1000.0).round() / 1000.0
    }
}

/// Scale every amount (per 100 g → portion).
pub fn scale(n: &Nut, f: f64) -> Nut {
    n.iter().map(|(k, v)| (k.clone(), round(v * f))).collect()
}

pub fn add(into: &mut Nut, n: &Nut) {
    for (k, v) in n {
        *into.entry(k.clone()).or_insert(0.0) += v;
    }
}

/// Keep only known keys with sane values.
pub fn clean(n: &Nut) -> Nut {
    n.iter()
        .filter_map(|(k, v)| {
            let d = def(k)?;
            (v.is_finite() && *v >= 0.0).then(|| (k.clone(), round(v.min(d.max))))
        })
        .collect()
}

/// Parse a nutrient map from AI/JSON input, accepting numbers or numeric strings.
pub fn from_json(v: &serde_json::Value) -> Nut {
    let mut out = Nut::new();
    if let Some(o) = v.as_object() {
        for d in CATALOG {
            if let Some(x) = o.get(d.key) {
                let n = match x {
                    serde_json::Value::Number(n) => n.as_f64(),
                    serde_json::Value::String(s) => s.trim().parse().ok(),
                    _ => None,
                };
                if let Some(n) = n.filter(|n| n.is_finite() && *n >= 0.0) {
                    out.insert(d.key.to_string(), round(n.min(d.max)));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        let mut keys: Vec<&str> = CATALOG.iter().map(|d| d.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), CATALOG.len(), "unique keys");
        for d in CATALOG {
            assert!(GROUPS.contains(&d.group), "{}", d.key);
            assert!(d.kind == Info || d.reference > 0.0, "{} needs a reference", d.key);
        }
        assert!(CATALOG.len() >= 40);
    }

    #[test]
    fn scale_clean_parse() {
        let mut n = Nut::new();
        n.insert("vitC".into(), 26.0);
        n.insert("fiber".into(), 6.5);
        let p = scale(&n, 1.5);
        assert_eq!((p["vitC"], p["fiber"]), (39.0, 9.8));
        let mut bad = Nut::new();
        bad.insert("bogus".into(), 1.0);
        bad.insert("sodium".into(), -1.0);
        bad.insert("iron".into(), 9999.0);
        let c = clean(&bad);
        assert_eq!(c.len(), 1);
        assert_eq!(c["iron"], 500.0);
        let j = serde_json::json!({"fiber": "4.25", "addedSugar": 12, "nope": 1, "vitD": null});
        let m = from_json(&j);
        assert_eq!((m["fiber"], m["addedSugar"], m.len()), (4.3, 12.0, 2));
        assert_eq!(round(0.12345), 0.123);
    }
}
