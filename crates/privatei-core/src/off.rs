//! Open Food Facts: packaged products, looked up online and kept in the user's food
//! database (source "off") so they work offline afterwards.
//!
//! Data is under the Open Database License (ODbL); keep it in its own source and credit
//! "Open Food Facts contributors".

use crate::fooddb::{NewFood, Portion, SourceInfo};
use crate::nutrients::Nut;
use serde_json::Value;
use std::time::Duration;

const UA: &str = "Private-I/0.5 (+https://github.com/sudo-panda/privatei)";

pub fn info() -> SourceInfo {
    SourceInfo {
        id: "off".into(),
        name: "Open Food Facts".into(),
        version: "looked up online".into(),
        licence: "Open Database License (ODbL) 1.0; contents under the Database Contents License".into(),
        attribution: "Product data © Open Food Facts contributors, openfoodfacts.org".into(),
        url: "https://world.openfoodfacts.org".into(),
        imported_at: 0,
        foods: 0,
    }
}

/// (OFF nutriment key without "_100g", catalog key, factor from OFF units to ours).
/// OFF stores vitamins and minerals in grams.
const MAP: &[(&str, &str, f64)] = &[
    ("fiber", "fiber", 1.0),
    ("carbohydrates", "carbs", 1.0),
    ("sugars", "sugar", 1.0),
    ("added-sugars", "addedSugar", 1.0),
    ("starch", "starch", 1.0),
    ("fat", "fat", 1.0),
    ("saturated-fat", "satFat", 1.0),
    ("monounsaturated-fat", "monoFat", 1.0),
    ("polyunsaturated-fat", "polyFat", 1.0),
    ("trans-fat", "transFat", 1.0),
    ("cholesterol", "cholesterol", 1000.0),
    ("alpha-linolenic-acid", "ala", 1.0),
    ("eicosapentaenoic-acid", "epa", 1.0),
    ("docosahexaenoic-acid", "dha", 1.0),
    ("sodium", "sodium", 1000.0),
    ("potassium", "potassium", 1000.0),
    ("calcium", "calcium", 1000.0),
    ("magnesium", "magnesium", 1000.0),
    ("phosphorus", "phosphorus", 1000.0),
    ("iron", "iron", 1000.0),
    ("zinc", "zinc", 1000.0),
    ("copper", "copper", 1000.0),
    ("manganese", "manganese", 1000.0),
    ("iodine", "iodine", 1_000_000.0),
    ("selenium", "selenium", 1_000_000.0),
    ("vitamin-a", "vitA", 1_000_000.0),
    ("vitamin-b1", "vitB1", 1000.0),
    ("vitamin-b2", "vitB2", 1000.0),
    ("vitamin-pp", "vitB3", 1000.0),
    ("pantothenic-acid", "vitB5", 1000.0),
    ("vitamin-b6", "vitB6", 1000.0),
    ("vitamin-b12", "b12", 1_000_000.0),
    ("vitamin-b9", "folate", 1_000_000.0),
    ("vitamin-c", "vitC", 1000.0),
    ("vitamin-d", "vitD", 1_000_000.0),
    ("vitamin-e", "vitE", 1000.0),
    ("vitamin-k", "vitK", 1_000_000.0),
    ("caffeine", "caffeine", 1000.0),
    ("alcohol", "alcohol", 1.0),
];

fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|x| x.is_finite() && *x >= 0.0)
}

/// Turn one OFF product (JSON) into a food. None without a name or energy.
pub fn product_to_food(p: &Value) -> Option<NewFood> {
    let code = p.get("code").and_then(Value::as_str)?.trim();
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let name = p.get("product_name").and_then(Value::as_str).unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }
    let brands = match p.get("brands") {
        Some(Value::Array(a)) => {
            a.iter().filter_map(Value::as_str).map(str::trim).filter(|b| !b.is_empty()).take(2).collect::<Vec<_>>().join(", ")
        }
        Some(Value::String(s)) => s.split(',').map(str::trim).take(2).collect::<Vec<_>>().join(", "),
        _ => String::new(),
    };
    let n = p.get("nutriments")?;
    let get = |k: &str| num(n.get(format!("{k}_100g").as_str()));
    let kcal = get("energy-kcal").or_else(|| get("energy-kj").map(|kj| kj / 4.184)).or_else(|| get("energy").map(|kj| kj / 4.184))?;
    let mut nut = Nut::new();
    for (k, key, f) in MAP {
        if let Some(v) = get(k) {
            nut.insert(key.to_string(), v * f);
        }
    }
    if !nut.contains_key("sodium") {
        if let Some(salt) = get("salt") {
            nut.insert("sodium".into(), salt / 2.5 * 1000.0);
        }
    }
    let mut portions = vec![];
    if let Some(g) = num(p.get("serving_quantity")).filter(|g| *g > 0.0 && *g < 5000.0) {
        portions.push(Portion { label: "1 serving".into(), grams: g });
    }
    let display = if brands.is_empty() || name.to_lowercase().contains(&brands.to_lowercase()) {
        name.to_string()
    } else {
        format!("{name} ({brands})")
    };
    Some(NewFood {
        source_id: code.to_string(),
        name: display.chars().take(120).collect(),
        category: "Packaged product".into(),
        kcal100: kcal,
        prot100: get("proteins").unwrap_or(0.0),
        nut100: nut,
        portions,
    })
}

const FIELDS: &str = "code,product_name,brands,nutriments,serving_quantity";

fn client() -> std::result::Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder().timeout(Duration::from_secs(20)).user_agent(UA).build().map_err(|e| e.to_string())
}

/// Search products by name (Swiss products first), or look one up by barcode.
/// Network only; the caller saves the results (see `Core::save_off`).
pub fn fetch(query: &str, base: &str) -> std::result::Result<Vec<NewFood>, String> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(vec![]);
    }
    let c = client()?;
    let offline = |_| "Couldn't reach Open Food Facts (offline?).".to_string();
    let mut products: Vec<Value> = vec![];
    if q.len() >= 8 && q.chars().all(|ch| ch.is_ascii_digit()) {
        let url = format!("{base}/api/v2/product/{q}.json?fields={FIELDS}");
        let v: Value = c.get(url).send().map_err(offline)?.json().map_err(|_| "Open Food Facts sent an unreadable answer.".to_string())?;
        if let Some(p) = v.get("product") {
            let mut p = p.clone();
            p["code"] = Value::String(q.to_string());
            products.push(p);
        }
    } else {
        let search_base =
            if base.contains("openfoodfacts.org") { "https://search.openfoodfacts.org".to_string() } else { base.to_string() };
        for filter in [" countries_tags:\"en:switzerland\"", ""] {
            let resp = c
                .get(format!("{search_base}/search"))
                .query(&[("q", format!("{q}{filter}")), ("page_size", "20".into()), ("fields", FIELDS.into())])
                .send()
                .map_err(offline)?;
            if resp.status().as_u16() == 429 {
                return Err("Open Food Facts is busy. Try again in a minute.".into());
            }
            let v: Value = resp.json().map_err(|_| "Open Food Facts sent an unreadable answer.".to_string())?;
            products.extend(v.get("hits").and_then(Value::as_array).cloned().unwrap_or_default());
            if products.len() >= 8 {
                break;
            }
        }
    }
    let mut foods: Vec<NewFood> = vec![];
    for p in &products {
        if let Some(f) = product_to_food(p) {
            if !foods.iter().any(|x| x.source_id == f.source_id) {
                foods.push(f);
            }
        }
    }
    foods.truncate(20);
    Ok(foods)
}

pub const BASE: &str = "https://world.openfoodfacts.org";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn product_mapping() {
        let p = json!({"code": "4316268627979", "product_name": "Skyr nature", "brands": ["Emmi"], "serving_quantity": "150",
            "nutriments": {"energy-kcal_100g": 62, "proteins_100g": 11, "sugars_100g": 4, "added-sugars_100g": 0,
                "salt_100g": 0.1, "calcium_100g": 0.12, "vitamin-d_100g": 0.0000005, "saturated-fat_100g": "0.2"}});
        let f = product_to_food(&p).unwrap();
        assert_eq!((f.source_id.as_str(), f.name.as_str(), f.kcal100, f.prot100), ("4316268627979", "Skyr nature (Emmi)", 62.0, 11.0));
        assert_eq!(f.nut100["addedSugar"], 0.0);
        assert_eq!(f.nut100["sodium"], 40.0, "from salt");
        assert!((f.nut100["calcium"] - 120.0).abs() < 1e-9);
        assert!((f.nut100["vitD"] - 0.5).abs() < 1e-9);
        assert_eq!(f.nut100["satFat"], 0.2);
        assert_eq!(f.portions[0].grams, 150.0);
        assert!(product_to_food(&json!({"code": "1", "product_name": "", "nutriments": {}})).is_none());
        assert!(product_to_food(&json!({"code": "1", "product_name": "No energy", "nutriments": {}})).is_none());
        assert!(product_to_food(&json!({"code": "x; drop", "product_name": "Bad code", "nutriments": {"energy-kcal_100g": 1}})).is_none());
        let kj = product_to_food(&json!({"code": "2", "product_name": "Only kJ", "nutriments": {"energy-kj_100g": 418.4}})).unwrap();
        assert!((kj.kcal100 - 100.0).abs() < 1e-9);
    }
}
