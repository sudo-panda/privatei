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
