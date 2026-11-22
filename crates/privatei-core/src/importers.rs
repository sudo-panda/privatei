//! Importers for national food composition databases (feature `import`).
//!
//! - USDA FoodData Central (SR Legacy + Foundation), CSV, public domain (CC0).
//! - Swiss Food Composition Database (FSVO), xlsx; free to use with the source acknowledged.
//!   Downloaded by the app from the official site, not shipped.

use crate::fooddb::{NewFood, Portion, SourceInfo};
use crate::nutrients::{self, Nut};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

pub fn usda_info(version: &str) -> SourceInfo {
    SourceInfo {
        id: "usda".into(),
        name: "USDA FoodData Central".into(),
        version: version.into(),
        licence: "Public domain (CC0 1.0)".into(),
        attribution: "U.S. Department of Agriculture, Agricultural Research Service. FoodData Central (SR Legacy, Foundation Foods). fdc.nal.usda.gov".into(),
        url: "https://fdc.nal.usda.gov/download-datasets/".into(),
        imported_at: 0,
        foods: 0,
    }
}

pub fn swiss_info(version: &str) -> SourceInfo {
    SourceInfo {
        id: "swiss".into(),
        name: "Swiss Food Composition Database".into(),
        version: version.into(),
        licence: "Free of charge, source must be acknowledged".into(),
        attribution: "Swiss Food Composition Database, Federal Food Safety and Veterinary Office FSVO. naehrwertdaten.ch".into(),
        url: "https://naehrwertdaten.ch/en/downloads/".into(),
        imported_at: 0,
        foods: 0,
    }
}

fn err(e: impl std::fmt::Display) -> String {
    format!("Import failed: {e}")
}

fn reader(path: &Path) -> Result<csv::Reader<std::fs::File>> {
    csv::ReaderBuilder::new().has_headers(true).flexible(true).from_path(path).map_err(|e| err(format!("{}: {e}", path.display())))
}

/// Read one or more unzipped FoodData Central CSV folders (SR Legacy, Foundation).
/// Only `sr_legacy_food` and `foundation_food` rows are kept.
pub fn usda_from_dirs(dirs: &[&Path]) -> Result<Vec<NewFood>> {
    // nutrient id -> (catalog key, preference index)
    let mut by_id: HashMap<u32, (&'static str, usize)> = HashMap::new();
    for d in nutrients::CATALOG {
        for (i, id) in d.usda.iter().enumerate() {
            by_id.insert(*id, (d.key, i));
        }
    }
    const KCAL: [u32; 3] = [1008, 2048, 2047];
    let mut out = vec![];
    for dir in dirs {
        let mut cats: HashMap<String, String> = HashMap::new();
        if let Ok(mut r) = reader(&dir.join("food_category.csv")) {
            for row in r.records().flatten() {
                cats.insert(row.get(0).unwrap_or("").into(), row.get(2).unwrap_or("").into());
            }
        }
        let mut units: HashMap<String, String> = HashMap::new();
        if let Ok(mut r) = reader(&dir.join("measure_unit.csv")) {
            for row in r.records().flatten() {
                units.insert(row.get(0).unwrap_or("").into(), row.get(1).unwrap_or("").into());
            }
        }
        // fdc_id -> food
        let mut foods: HashMap<String, NewFood> = HashMap::new();
        let mut order: Vec<String> = vec![];
        for row in reader(&dir.join("food.csv"))?.records().flatten() {
            let kind = row.get(1).unwrap_or("");
            if kind != "sr_legacy_food" && kind != "foundation_food" {
                continue;
            }
            let id = row.get(0).unwrap_or("").to_string();
            let cat = row.get(3).and_then(|c| cats.get(c)).cloned().unwrap_or_default();
            order.push(id.clone());
            foods.insert(id.clone(), NewFood { source_id: id, name: row.get(2).unwrap_or("").into(), category: cat, ..Default::default() });
        }
        let mut kcal_pref: HashMap<String, usize> = HashMap::new();
        let mut pref: HashMap<(String, &'static str), usize> = HashMap::new();
        for row in reader(&dir.join("food_nutrient.csv"))?.records().flatten() {
            let fid = row.get(1).unwrap_or("");
            let Some(f) = foods.get_mut(fid) else { continue };
            let Ok(nid) = row.get(2).unwrap_or("").parse::<u32>() else { continue };
            let Ok(amount) = row.get(3).unwrap_or("").parse::<f64>() else { continue };
            if !amount.is_finite() || amount < 0.0 {
                continue;
            }
            if nid == 1003 {
                f.prot100 = amount;
            } else if let Some(i) = KCAL.iter().position(|k| *k == nid) {
                let best = kcal_pref.entry(fid.to_string()).or_insert(usize::MAX);
                if i < *best {
                    *best = i;
                    f.kcal100 = amount;
                }
            } else if let Some((key, i)) = by_id.get(&nid) {
                let best = pref.entry((fid.to_string(), key)).or_insert(usize::MAX);
                if *i < *best {
                    *best = *i;
                    f.nut100.insert(key.to_string(), amount);
                }
            }
        }
        if let Ok(mut r) = reader(&dir.join("food_portion.csv")) {
            for row in r.records().flatten() {
                let Some(f) = foods.get_mut(row.get(1).unwrap_or("")) else { continue };
                let Ok(g) = row.get(7).unwrap_or("").parse::<f64>() else { continue };
                let amount = row.get(3).unwrap_or("").trim();
                let unit = units.get(row.get(4).unwrap_or("")).map(String::as_str).filter(|u| *u != "undetermined").unwrap_or("");
                let desc = row.get(5).unwrap_or("").trim();
                let modifier = row.get(6).unwrap_or("").trim();
                let what = [unit, desc, modifier].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ");
                let amount = amount.trim_end_matches(".0");
                let label =
                    if amount.is_empty() || amount == "1" && !what.is_empty() { format!("1 {what}") } else { format!("{amount} {what}") };
                f.portions.push(Portion { label: label.trim().into(), grams: g });
            }
        }
        out.extend(order.into_iter().filter_map(|id| foods.remove(&id)).filter(|f| f.kcal100 > 0.0 || f.prot100 > 0.0));
    }
    Ok(out)
}

pub fn cofid_info(version: &str) -> SourceInfo {
    SourceInfo {
        id: "cofid".into(),
        name: "UK CoFID (McCance and Widdowson)".into(),
        version: version.into(),
        licence: "Open Government Licence v3.0".into(),
        attribution: "Contains public sector information licensed under the Open Government Licence v3.0. McCance and Widdowson's Composition of Foods Integrated Dataset, Public Health England / OHID.".into(),
        url: "https://www.gov.uk/government/publications/composition-of-foods-integrated-dataset-cofid".into(),
        imported_at: 0,
        foods: 0,
    }
}

/// CoFID values: numbers, "Tr" (trace), "N" (present, amount unknown), blank.
fn cofid_value(s: &str) -> Option<f64> {
    let t = s.trim();
    match t {
        "" | "N" | "n" => None,
        "Tr" | "tr" | "TR" => Some(0.0),
        _ => t.trim_start_matches('(').trim_end_matches(')').parse().ok().filter(|x: &f64| x.is_finite() && *x >= 0.0),
    }
}

fn cofid_group(code: &str) -> &'static str {
    match code.chars().next() {
        Some('A') => "Cereals and cereal products",
        Some('B') => "Milk and milk products",
        Some('C') => "Eggs",
        Some('D') => "Vegetables",
        Some('F') => "Fruit",
        Some('G') => "Nuts and seeds",
        Some('H') => "Herbs and spices",
        Some('J') => "Fish and fish products",
        Some('M') => "Meat and meat products",
        Some('O') => "Fats and oils",
        Some('P') => "Beverages",
        Some('Q') => "Alcoholic beverages",
        Some('S') => "Sugars, preserves and snacks",
        Some('W') => "Soups, sauces and miscellaneous",
        _ => "",
    }
}

/// (sheet name prefix, column header, catalog key; "_kcal"/"_prot" for energy and protein).
const COFID_COLUMNS: &[(&str, &str, &str)] = &[
    ("1.3", "Energy (kcal) (kcal)", "_kcal"),
    ("1.3", "Protein (g)", "_prot"),
    ("1.3", "Water (g)", "water"),
    ("1.3", "Fat (g)", "fat"),
    ("1.3", "Carbohydrate (g)", "carbs"),
    ("1.3", "Starch (g)", "starch"),
    ("1.3", "Total sugars (g)", "sugar"),
    ("1.3", "Alcohol (g)", "alcohol"),
    ("1.3", "AOAC fibre (g)", "fiber"),
    ("1.3", "Satd FA /100g fd (g)", "satFat"),
    ("1.3", "Mono FA /100g food (g)", "monoFat"),
    ("1.3", "Poly FA /100g food (g)", "polyFat"),
    ("1.3", "Trans FAs /100g food (g)", "transFat"),
    ("1.3", "Cholesterol (mg)", "cholesterol"),
    ("1.4", "Sodium (mg)", "sodium"),
    ("1.4", "Potassium (mg)", "potassium"),
    ("1.4", "Calcium (mg)", "calcium"),
    ("1.4", "Magnesium (mg)", "magnesium"),
    ("1.4", "Phosphorus (mg)", "phosphorus"),
    ("1.4", "Iron (mg)", "iron"),
    ("1.4", "Copper (mg)", "copper"),
    ("1.4", "Zinc (mg)", "zinc"),
    ("1.4", "Manganese (mg)", "manganese"),
    ("1.4", "Selenium (µg)", "selenium"),
    ("1.4", "Iodine (µg)", "iodine"),
    ("1.5", "Retinol Equivalent (µg)", "vitA"),
    ("1.5", "Vitamin D (µg)", "vitD"),
    ("1.5", "Vitamin E (mg)", "vitE"),
    ("1.5", "Vitamin K1 (µg)", "vitK"),
    ("1.5", "Thiamin (mg)", "vitB1"),
    ("1.5", "Riboflavin (mg)", "vitB2"),
    ("1.5", "Niacin (mg)", "vitB3"),
    ("1.5", "Vitamin B6 (mg)", "vitB6"),
    ("1.5", "Vitamin B12 (µg)", "b12"),
    ("1.5", "Folate (µg)", "folate"),
    ("1.5", "Pantothenate (mg)", "vitB5"),
    ("1.5", "Vitamin C (mg)", "vitC"),
    ("1.12", "cis n-3 C18:3 /100g food (g)", "ala"),
    ("1.12", "cis n-3 C20:5 /100g food (g)", "epa"),
    ("1.12", "cis n-3 C22:6 /100g food (g)", "dha"),
];

/// Parse the CoFID 2021 workbook: sheets joined by food code.
pub fn cofid_from_xlsx(bytes: &[u8]) -> Result<Vec<NewFood>> {
    use calamine::{Data, Reader, Xlsx};
    let mut wb: Xlsx<_> = Xlsx::new(std::io::Cursor::new(bytes)).map_err(err)?;
    let text = |c: &Data| match c {
        Data::String(s) => s.clone(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        _ => String::new(),
    };
    let mut foods: HashMap<String, NewFood> = HashMap::new();
    let mut order: Vec<String> = vec![];
    for sheet in wb.sheet_names().to_vec() {
        let prefix = sheet.split_whitespace().next().unwrap_or("").to_string();
        let cols: Vec<&(&str, &str, &str)> = COFID_COLUMNS.iter().filter(|(p, _, _)| *p == prefix).collect();
        if cols.is_empty() {
            continue;
        }
        let range = wb.worksheet_range(&sheet).map_err(err)?;
        let rows: Vec<Vec<String>> = range.rows().map(|r| r.iter().map(text).collect()).collect();
        let Some(hi) = rows.iter().position(|r| r.get(1).map(|c| c.trim()) == Some("Food Name")) else { continue };
        let head = &rows[hi];
        let idx: Vec<(usize, &str)> =
            cols.iter().filter_map(|(_, name, key)| head.iter().position(|h| h.trim() == *name).map(|i| (i, *key))).collect();
        let c_group = head.iter().position(|h| h.trim() == "Group");
        for r in &rows[hi + 1..] {
            let code = r.first().map(|c| c.trim()).unwrap_or("");
            let name = r.get(1).map(|c| c.trim()).unwrap_or("");
            if code.is_empty() || name.is_empty() {
                continue;
            }
            let f = foods.entry(code.to_string()).or_insert_with(|| {
                order.push(code.to_string());
                NewFood {
                    source_id: code.to_string(),
                    name: name.to_string(),
                    category: c_group.and_then(|i| r.get(i)).map(|g| cofid_group(g.trim()).to_string()).unwrap_or_default(),
                    ..Default::default()
                }
            });
            for (i, key) in &idx {
                let Some(v) = r.get(*i).and_then(|c| cofid_value(c)) else { continue };
                match *key {
                    "_kcal" => f.kcal100 = v,
                    "_prot" => f.prot100 = v,
                    k => {
                        f.nut100.insert(k.to_string(), v);
                    }
                }
            }
        }
    }
    let out: Vec<NewFood> = order.into_iter().filter_map(|c| foods.remove(&c)).filter(|f| f.kcal100 > 0.0 || f.prot100 > 0.0).collect();
    if out.is_empty() {
        return Err(err("no foods found in the CoFID file"));
    }
    Ok(out)
}

/// A cell value from the Swiss sheet: numbers, "n.d." (unknown), "tr." (trace), "<0.5".
fn swiss_value(s: &str) -> Option<f64> {
    let t = s.trim();
    match t {
        "" | "-" | "n.d." | "n.d" => None,
        "tr." | "tr" => Some(0.0),
        _ if t.starts_with('<') => Some(0.0),
        _ => t.replace(',', ".").parse().ok().filter(|x: &f64| x.is_finite() && *x >= 0.0),
    }
}

/// Parse the Swiss Food Composition Database xlsx (English edition).
pub fn swiss_from_xlsx(bytes: &[u8]) -> Result<Vec<NewFood>> {
    use calamine::{Data, Reader, Xlsx};
    let mut wb: Xlsx<_> = Xlsx::new(std::io::Cursor::new(bytes)).map_err(err)?;
    let names = wb.sheet_names().to_vec();
    for sheet in names {
        let Ok(range) = wb.worksheet_range(&sheet) else { continue };
        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|r| {
                r.iter()
                    .map(|c| match c {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    })
                    .collect()
            })
            .collect();
        let Some(hi) = rows.iter().position(|r| r.first().map(|c| c.trim()) == Some("ID") && r.iter().any(|c| c.trim() == "Name")) else {
            continue;
        };
        let head = &rows[hi];
        let col = |name: &str| head.iter().position(|c| c.trim() == name);
        let (Some(c_id), Some(c_name)) = (col("ID"), col("Name")) else { continue };
        let c_cat = col("Category");
        let c_kcal = col("Energy, kilocalories (kcal)").ok_or_else(|| err("no kcal column"))?;
        let c_prot = col("Protein (g)").ok_or_else(|| err("no protein column"))?;
        let nut_cols: Vec<(&'static str, usize)> =
            nutrients::CATALOG.iter().filter(|d| !d.swiss.is_empty()).filter_map(|d| col(d.swiss).map(|i| (d.key, i))).collect();
        let mut out = vec![];
        for r in &rows[hi + 1..] {
            let get = |i: usize| r.get(i).map(String::as_str).unwrap_or("");
            let name = get(c_name).trim();
            let id = get(c_id).trim();
            if name.is_empty() || id.is_empty() {
                continue;
            }
            let Some(kcal) = swiss_value(get(c_kcal)) else { continue };
            let mut nut = Nut::new();
            for (k, i) in &nut_cols {
                if let Some(v) = swiss_value(get(*i)) {
                    nut.insert(k.to_string(), v);
                }
            }
            let category = c_cat.map(get).unwrap_or("").split(';').next().unwrap_or("").replace('/', " · ");
            out.push(NewFood {
                source_id: id.trim_end_matches(".0").into(),
                name: name.into(),
                category,
                kcal100: kcal,
                prot100: swiss_value(get(c_prot)).unwrap_or(0.0),
                nut100: nut,
                portions: vec![],
            });
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }
    Err(err("no food table found in the file"))
}

/// Find the current English xlsx on the official downloads page and fetch it.
/// Returns (bytes, version label).
pub fn download_swiss() -> Result<(Vec<u8>, String)> {
    const PAGE: &str = "https://naehrwertdaten.ch/en/downloads/";
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("Private-I food tracker")
        .build()
        .map_err(err)?;
    let page = client
        .get(PAGE)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|_| "Couldn't reach naehrwertdaten.ch (offline?).".to_string())?;
    let url = page
        .split('"')
        .find(|s| s.starts_with("https://naehrwertdaten.ch/") && s.ends_with(".xlsx") && s.contains("Swiss_food_composition_database"))
        .ok_or("Couldn't find the database file on naehrwertdaten.ch — the site may have changed.")?
        .to_string();
    // ".../uploads/2026/07/Swiss_food_composition_database.xlsx" -> "2026-07"
    let version = url.split("/uploads/").nth(1).map(|p| p.split('/').take(2).collect::<Vec<_>>().join("-")).unwrap_or_default();
    let mut resp =
        client.get(&url).send().and_then(|r| r.error_for_status()).map_err(|_| "The download failed. Try again later.".to_string())?;
    let mut bytes = vec![];
    resp.read_to_end(&mut bytes).map_err(err)?;
    if bytes.len() > 50 * 1024 * 1024 {
        return Err("The downloaded file is unexpectedly large.".into());
    }
    Ok((bytes, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cofid_values() {
        assert_eq!(cofid_value("12.5"), Some(12.5));
        assert_eq!(cofid_value("Tr"), Some(0.0));
        assert_eq!(cofid_value("N"), None);
        assert_eq!(cofid_value(""), None);
        assert_eq!(cofid_value("(1.2)"), Some(1.2));
        assert_eq!(cofid_group("DG"), "Vegetables");
    }

    #[test]
    fn swiss_values() {
        assert_eq!(swiss_value("12.5"), Some(12.5));
        assert_eq!(swiss_value("n.d."), None);
        assert_eq!(swiss_value("tr."), Some(0.0));
        assert_eq!(swiss_value("<0.5"), Some(0.0));
        assert_eq!(swiss_value("-"), None);
        assert_eq!(swiss_value("abc"), None);
    }

    #[test]
    fn usda_csv() {
        let dir = std::env::temp_dir().join(format!("privatei-usda-{}", crate::store::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let w = |name: &str, body: &str| std::fs::write(dir.join(name), body).unwrap();
        w("food.csv", "\"fdc_id\",\"data_type\",\"description\",\"food_category_id\",\"publication_date\"\n\"1\",\"sr_legacy_food\",\"Lentils, cooked\",\"16\",\"2019\"\n\"2\",\"sample_food\",\"SKIP ME\",\"16\",\"2019\"\n\"3\",\"foundation_food\",\"Hummus, commercial\",\"16\",\"2019\"\n");
        w("food_category.csv", "\"id\",\"code\",\"description\"\n\"16\",\"1600\",\"Legumes and Legume Products\"\n");
        w("measure_unit.csv", "\"id\",\"name\"\n\"1000\",\"cup\"\n\"9999\",\"undetermined\"\n");
        w(
            "food_nutrient.csv",
            "\"id\",\"fdc_id\",\"nutrient_id\",\"amount\"\n\"a\",\"1\",\"1008\",\"116\"\n\"b\",\"1\",\"1003\",\"9.02\"\n\"c\",\"1\",\"1079\",\"7.9\"\n\"d\",\"1\",\"1177\",\"181\"\n\"e\",\"1\",\"1190\",\"180\"\n\"f\",\"3\",\"2047\",\"229\"\n\"g\",\"3\",\"1003\",\"7.35\"\n\"h\",\"2\",\"1008\",\"1\"\n",
        );
        w("food_portion.csv", "\"id\",\"fdc_id\",\"seq_num\",\"amount\",\"measure_unit_id\",\"portion_description\",\"modifier\",\"gram_weight\"\n\"p\",\"1\",\"1\",\"1\",\"1000\",\"\",\"\",\"198\"\n\"q\",\"3\",\"1\",\"2.0\",\"9999\",\"\",\"tbsp\",\"30\"\n");
        let foods = usda_from_dirs(&[&dir]).unwrap();
        assert_eq!(foods.len(), 2);
        let l = &foods[0];
        assert_eq!(
            (l.name.as_str(), l.kcal100, l.prot100, l.category.as_str()),
            ("Lentils, cooked", 116.0, 9.02, "Legumes and Legume Products")
        );
        assert_eq!(l.nut100["fiber"], 7.9);
        assert_eq!(l.nut100["folate"], 180.0, "DFE preferred over total folate");
        assert_eq!(l.portions[0], Portion { label: "1 cup".into(), grams: 198.0 });
        assert_eq!(foods[1].kcal100, 229.0, "Atwater energy used when 1008 is missing");
        assert_eq!(foods[1].portions[0].label, "2 tbsp");
    }
}
