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

// ---------- Swiss offline pack ----------
//
// A monthly GitHub Action (`.github/workflows/off-pack.yml`) turns OFF's worldwide CSV
// export into `off-ch.jsonl.gz`: one header line, then one trimmed OFF product per line,
// for products sold in Switzerland. The app downloads it and builds its own read-only
// database file from it (`PACK_FILE`), so the pack is checked by the same
// `product_to_food` as online answers.

/// Where the app downloads the pack (a pre-release asset, so app updates never see it).
pub const PACK_URL: &str = "https://github.com/sudo-panda/privatei/releases/download/off-ch-pack/off-ch.jsonl.gz";
/// The pack's database file, next to `foods-user.sqlite`.
pub const PACK_FILE: &str = "foods-off-ch.sqlite";
/// Its source id: refs look like "offch:7610000000011".
pub const PACK_SOURCE: &str = "offch";
/// Packaged products (online lookups and the Swiss pack), as opposed to generic foods.
pub fn is_product(source: &str) -> bool {
    source == "off" || source == PACK_SOURCE
}

const MAX_PACK_LINES: usize = 1_000_000;

pub fn pack_info(date: &str) -> SourceInfo {
    SourceInfo {
        id: PACK_SOURCE.into(),
        name: "Open Food Facts: Swiss products".into(),
        version: date.into(),
        licence: "Open Database License (ODbL) 1.0; contents under the Database Contents License".into(),
        attribution:
            "Product data © Open Food Facts contributors, openfoodfacts.org. Products sold in Switzerland, from the Open Food Facts export"
                .into(),
        url: "https://world.openfoodfacts.org/data".into(),
        imported_at: 0,
        foods: 0,
    }
}

/// OFF nutriment columns kept in the pack (everything `product_to_food` reads).
#[cfg(feature = "import")]
fn pack_nutrients() -> Vec<&'static str> {
    let mut v: Vec<&str> = MAP.iter().map(|(k, _, _)| *k).collect();
    v.extend(["energy-kcal", "energy-kj", "energy", "proteins", "salt"]);
    v
}

/// Turn OFF's tab-separated CSV export into a pack (uncompressed lines to `out`).
/// Keeps products sold in Switzerland that `product_to_food` accepts. Returns how many.
#[cfg(feature = "import")]
pub fn csv_to_pack(csv: impl std::io::Read, mut out: impl std::io::Write, date: &str) -> std::result::Result<usize, String> {
    let e = |e: &dyn std::fmt::Display| format!("Pack build failed: {e}");
    let mut rd = csv::ReaderBuilder::new().delimiter(b'\t').quoting(false).flexible(true).from_reader(csv);
    let headers = rd.headers().map_err(|x| e(&x))?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let (code, name, brands, countries, serving) = (
        col("code").ok_or_else(|| e(&"no code column"))?,
        col("product_name").ok_or_else(|| e(&"no product_name column"))?,
        col("brands"),
        col("countries_tags").ok_or_else(|| e(&"no countries_tags column"))?,
        col("serving_quantity"),
    );
    let nuts: Vec<(String, usize)> =
        pack_nutrients().into_iter().filter_map(|k| col(&format!("{k}_100g")).map(|i| (format!("{k}_100g"), i))).collect();
    writeln!(out, "{}", serde_json::json!({"pack": "off-ch", "date": date})).map_err(|x| e(&x))?;
    let mut n = 0;
    let mut seen = std::collections::HashSet::new();
    let mut rec = csv::StringRecord::new();
    loop {
        match rd.read_record(&mut rec) {
            Ok(false) => break,
            Ok(true) => {}
            Err(_) => continue, // a broken line in the export: skip it
        }
        let get = |i: usize| rec.get(i).unwrap_or("").trim();
        if !get(countries).split(',').any(|c| c.trim() == "en:switzerland") {
            continue;
        }
        let mut nutriments = serde_json::Map::new();
        for (k, i) in &nuts {
            if let Some(v) = get(*i).parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0) {
                nutriments.insert(k.clone(), serde_json::json!(v));
            }
        }
        let mut p = serde_json::json!({"code": get(code), "product_name": get(name), "nutriments": nutriments});
        if let Some(b) = brands.map(get).filter(|b| !b.is_empty()) {
            p["brands"] = Value::String(b.to_string());
        }
        if let Some(sq) = serving.map(get).and_then(|x| x.parse::<f64>().ok()) {
            p["serving_quantity"] = serde_json::json!(sq);
        }
        if product_to_food(&p).is_none() || !seen.insert(get(code).to_string()) {
            continue;
        }
        writeln!(out, "{p}").map_err(|x| e(&x))?;
        n += 1;
    }
    Ok(n)
}

/// Build the pack's database file at `out` from pack lines. Returns (products, pack date).
pub fn pack_to_db(pack: impl std::io::BufRead, out: &std::path::Path) -> std::result::Result<(usize, String), String> {
    let bad = || "The Swiss products file is damaged. Try again later.".to_string();
    let mut lines = pack.lines();
    let head: Value = lines.next().and_then(|l| l.ok()).and_then(|l| serde_json::from_str(&l).ok()).ok_or_else(bad)?;
    if head.get("pack").and_then(Value::as_str) != Some("off-ch") {
        return Err(bad());
    }
    let date: String =
        head.get("date").and_then(Value::as_str).unwrap_or("").chars().filter(|c| c.is_ascii_digit() || *c == '-').take(10).collect();
    let mut foods = vec![];
    for line in lines.take(MAX_PACK_LINES) {
        let line = line.map_err(|_| bad())?;
        if let Some(f) = serde_json::from_str::<Value>(&line).ok().as_ref().and_then(product_to_food) {
            foods.push(f);
        }
    }
    if foods.is_empty() {
        return Err(bad());
    }
    let db = crate::fooddb::FoodDb::create(Some(out))?;
    let n = db.replace_source(&pack_info(&date), &foods)?;
    drop(db);
    crate::fooddb::vacuum(out)?;
    Ok((n, date))
}

/// Download the pack from `url` and build its database file at `out`. Network only:
/// call it without holding the Core lock, then `Core::install_off_pack`.
#[cfg(feature = "import")]
pub fn download_pack(url: &str, out: &std::path::Path) -> std::result::Result<(usize, String), String> {
    use std::io::Read;
    let c = reqwest::blocking::Client::builder().timeout(Duration::from_secs(600)).user_agent(UA).build().map_err(|e| e.to_string())?;
    let resp = c.get(url).send().map_err(|_| "Couldn't reach GitHub to download the Swiss products (offline?).".to_string())?;
    if !resp.status().is_success() {
        return Err("The Swiss products file isn't available right now. Try again later.".into());
    }
    // At most 100 MB compressed and 1 GB unpacked.
    let gz = flate2::read::GzDecoder::new(resp.take(100 << 20));
    let r = pack_to_db(std::io::BufReader::new(gz.take(1 << 30)), out);
    if r.is_err() {
        let _ = std::fs::remove_file(out);
    }
    r
}

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

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("privatei-offpack-{tag}-{}", crate::store::new_id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const PACK: &str = concat!(
        "{\"pack\":\"off-ch\",\"date\":\"2026-11-23\"}\n",
        "{\"code\":\"7610000000011\",\"product_name\":\"Skyr nature\",\"brands\":\"Migros\",\"serving_quantity\":150,\"nutriments\":{\"energy-kcal_100g\":62,\"proteins_100g\":11,\"salt_100g\":0.1}}\n",
        "not json\n",
        "{\"code\":\"7610000000028\",\"product_name\":\"\",\"nutriments\":{\"energy-kcal_100g\":1}}\n",
        "{\"code\":\"7610000000035\",\"product_name\":\"Pumpernickel\",\"brands\":\"Coop Naturaplan\",\"nutriments\":{\"energy-kj_100g\":836.8,\"fiber_100g\":8}}\n",
    );

    #[cfg(feature = "import")]
    #[test]
    fn csv_to_pack_keeps_swiss_products() {
        let csv =
            "code\tproduct_name\tbrands\tcountries_tags\tserving_quantity\tenergy-kcal_100g\tproteins_100g\tsalt_100g\tvitamin-d_100g\n\
            7610000000011\tSkyr nature\tMigros\ten:france,en:switzerland\t150\t62\t11\t0.1\t0.0000005\n\
            3000000000000\tCroissant\tX\ten:france\t\t400\t8\t1\t\n\
            7610000000028\t\tCoop\ten:switzerland\t\t100\t1\t\t\n\
            7610000000035\tNo energy\tCoop\ten:switzerland\t\t\t1\t\t\n\
            broken line\n\
            7610000000011\tSkyr nature again\tMigros\ten:switzerland\t\t62\t11\t\t\n\
            7610000000042\t\"Quoted\" bar\tCoop\ten:switzerland\t\t-5\t20\t\t\n";
        let mut out = vec![];
        let n = csv_to_pack(csv.as_bytes(), &mut out, "2026-11-23").unwrap();
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(n, 1, "only Swiss products with a name and energy, once each: {text}");
        assert_eq!(lines[0], r#"{"date":"2026-11-23","pack":"off-ch"}"#);
        let f = product_to_food(&serde_json::from_str(lines[1]).unwrap()).unwrap();
        assert_eq!((f.source_id.as_str(), f.name.as_str(), f.kcal100), ("7610000000011", "Skyr nature (Migros)", 62.0));
        assert_eq!(f.nut100["sodium"], 40.0);
        assert!((f.nut100["vitD"] - 0.5).abs() < 1e-9);
        assert_eq!(f.portions[0].grams, 150.0);
        // Without the columns it needs, it refuses rather than writing an empty pack.
        assert!(csv_to_pack("code\tname\n".as_bytes(), vec![], "2026-11-23").is_err());
    }

    #[test]
    fn pack_to_db_builds_a_searchable_file() {
        let dir = tmp("db");
        let out = dir.join(PACK_FILE);
        let (n, date) = pack_to_db(PACK.as_bytes(), &out).unwrap();
        assert_eq!((n, date.as_str()), (2, "2026-11-23"), "bad and nameless lines skipped");
        let mut db = crate::fooddb::FoodDb::create(None).unwrap();
        db.attach(&out).unwrap();
        let hit = &db.search("pumpernickel", 5)[0];
        assert_eq!((hit.reference.as_str(), hit.source_name.as_str()), ("offch:7610000000035", "Open Food Facts: Swiss products"));
        assert!((hit.kcal100 - 200.0).abs() < 0.1);
        assert!(db.sources().iter().any(|s| s.id == PACK_SOURCE && s.version == "2026-11-23" && s.foods == 2));
        assert_eq!(db.detach("usda"), None, "only pack files are detached");
        assert_eq!(db.detach(PACK_SOURCE), Some(out.clone()));
        assert!(db.search("pumpernickel", 5).is_empty());
        // Not a pack: wrong header, or nothing usable.
        assert!(pack_to_db("{\"pack\":\"other\"}\n".as_bytes(), &dir.join("x.sqlite")).is_err());
        assert!(pack_to_db("{\"pack\":\"off-ch\",\"date\":\"2026-11-23\"}\nnope\n".as_bytes(), &dir.join("y.sqlite")).is_err());
        assert!(pack_to_db("".as_bytes(), &dir.join("z.sqlite")).is_err());
        // A file that isn't a food database can't be attached.
        std::fs::write(dir.join("junk.sqlite"), "junk").unwrap();
        assert!(db.attach(&dir.join("junk.sqlite")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(feature = "import")]
    #[test]
    fn download_pack_over_http() {
        use std::io::{Read, Write};
        let gz = {
            let mut e = flate2::write::GzEncoder::new(vec![], flate2::Compression::fast());
            e.write_all(PACK.as_bytes()).unwrap();
            e.finish().unwrap()
        };
        let serve = |status: &'static str, body: Vec<u8>| {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/off-ch.jsonl.gz", l.local_addr().unwrap());
            std::thread::spawn(move || {
                let (mut s, _) = l.accept().unwrap();
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(&body);
            });
            url
        };
        let dir = tmp("dl");
        let out = dir.join("pack.part");
        assert_eq!(download_pack(&serve("200 OK", gz), &out).unwrap().0, 2);
        assert!(out.is_file());
        let bad = dir.join("bad.part");
        assert!(download_pack(&serve("404 Not Found", b"missing".to_vec()), &bad).is_err());
        assert!(download_pack(&serve("200 OK", b"not gzip".to_vec()), &bad).is_err());
        assert!(!bad.exists(), "a failed download leaves no file behind");
        let _ = std::fs::remove_dir_all(dir);
    }
}
