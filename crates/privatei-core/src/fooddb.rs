//! The food database: national food composition tables, searchable offline.
//!
//! Each database file holds one or more sources (USDA, Swiss, …). The app ships a
//! read-only file and keeps a second, writable one for downloaded sources; search
//! looks at both. Values are per 100 g edible portion; a missing nutrient key means
//! "not in this database", never zero.

use crate::nutrients::{self, Nut};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    format!("Food database error: {e}")
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS source (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, version TEXT, licence TEXT, attribution TEXT,
  url TEXT, imported_at INTEGER, foods INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS food (
  id INTEGER PRIMARY KEY, source TEXT NOT NULL, source_id TEXT NOT NULL, name TEXT NOT NULL,
  category TEXT, kcal REAL, prot REAL, nut TEXT NOT NULL DEFAULT '{}', UNIQUE(source, source_id));
CREATE TABLE IF NOT EXISTS portion (food_id INTEGER NOT NULL, label TEXT NOT NULL, grams REAL NOT NULL);
CREATE INDEX IF NOT EXISTS portion_food ON portion(food_id);
CREATE VIRTUAL TABLE IF NOT EXISTS food_fts USING fts5(
  name, category, content='food', content_rowid='id', tokenize='unicode61 remove_diacritics 2');
";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub licence: String,
    pub attribution: String,
    pub url: String,
    pub imported_at: i64,
    pub foods: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Portion {
    pub label: String,
    pub grams: f64,
}

/// A food from a database, per 100 g.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DbFood {
    /// "usda:167512", "swiss:A123" — stable across re-imports.
    #[serde(rename = "ref")]
    pub reference: String,
    pub name: String,
    pub source: String,
    pub source_name: String,
    pub category: String,
    pub kcal100: f64,
    pub prot100: f64,
    pub fiber100: Option<f64>,
    /// How many catalog nutrients this entry has values for.
    pub known: usize,
    pub nut100: Nut,
    pub portions: Vec<Portion>,
}

/// One database file.
struct Part {
    conn: Connection,
    writable: bool,
}

pub struct FoodDb {
    parts: Vec<Part>,
}

impl FoodDb {
    /// `bundled`: the read-only file shipped with the app (optional);
    /// `user`: the writable file for downloaded sources (created if missing).
    pub fn open(bundled: Option<&Path>, user: &Path) -> Result<FoodDb> {
        let mut parts = vec![];
        if let Some(b) = bundled.filter(|p| p.is_file()) {
            let conn = Connection::open_with_flags(b, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
            parts.push(Part { conn, writable: false });
        }
        if let Some(dir) = user.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        let conn = Connection::open(user).map_err(err)?;
        conn.execute_batch(SCHEMA).map_err(err)?;
        parts.push(Part { conn, writable: true });
        Ok(FoodDb { parts })
    }

    /// An empty, writable database (tests, building the bundled file).
    pub fn create(path: Option<&Path>) -> Result<FoodDb> {
        let conn = match path {
            Some(p) => {
                let _ = std::fs::remove_file(p);
                Connection::open(p).map_err(err)?
            }
            None => Connection::open_in_memory().map_err(err)?,
        };
        conn.execute_batch(SCHEMA).map_err(err)?;
        Ok(FoodDb { parts: vec![Part { conn, writable: true }] })
    }

    fn writable(&self) -> Result<&Connection> {
        self.parts.iter().rev().find(|p| p.writable).map(|p| &p.conn).ok_or_else(|| err("no writable database"))
    }

    /// Every source across all files.
    pub fn sources(&self) -> Vec<SourceInfo> {
        let mut out = vec![];
        for p in &self.parts {
            let Ok(mut st) = p.conn.prepare("SELECT id, name, IFNULL(version,''), IFNULL(licence,''), IFNULL(attribution,''), IFNULL(url,''), IFNULL(imported_at,0), IFNULL(foods,0) FROM source ORDER BY name") else { continue };
            let rows = st.query_map([], |r| {
                Ok(SourceInfo {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    version: r.get(2)?,
                    licence: r.get(3)?,
                    attribution: r.get(4)?,
                    url: r.get(5)?,
                    imported_at: r.get(6)?,
                    foods: r.get(7)?,
                })
            });
            if let Ok(rows) = rows {
                out.extend(rows.flatten());
            }
        }
        out
    }

    /// Search by name. Every word must match the start of a word in the name or
    /// category ("chick bre" finds "Chicken, breast"). Best matches first.
    pub fn search(&self, query: &str, limit: usize) -> Vec<DbFood> {
        let words: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .take(8)
            .map(|w| {
                let w = w.replace('"', "");
                let alts = synonyms(&w.to_lowercase());
                if alts.is_empty() {
                    format!("\"{w}\"*")
                } else {
                    let mut all = vec![format!("\"{w}\"*")];
                    all.extend(alts.iter().map(|a| format!("\"{a}\"*")));
                    format!("({})", all.join(" OR "))
                }
            })
            .collect();
        if words.is_empty() {
            return vec![];
        }
        let fts = words.join(" AND ");
        let lower: Vec<String> =
            query.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(String::from).collect();
        let mut hits: Vec<(f64, DbFood)> = vec![];
        for p in &self.parts {
            let sql = "SELECT f.id, bm25(food_fts, 4.0, 1.0) FROM food_fts JOIN food f ON f.id = food_fts.rowid
                       WHERE food_fts MATCH ?1 ORDER BY bm25(food_fts, 4.0, 1.0) LIMIT 60";
            let Ok(mut st) = p.conn.prepare(sql) else { continue };
            let ids: Vec<(i64, f64)> =
                st.query_map([&fts], |r| Ok((r.get(0)?, r.get(1)?))).map(|r| r.flatten().collect()).unwrap_or_default();
            for (id, rank) in ids {
                if let Some(f) = load(&p.conn, id) {
                    // Rescore: bm25 (lower is better) + prefer names that start with the
                    // first word, shorter names, and entries with more nutrient data.
                    let name = f.name.to_lowercase();
                    let name_words: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
                    let mut score = -rank;
                    if lower.first().is_some_and(|w| name.starts_with(w.as_str())) {
                        score += 3.0;
                    }
                    // Whole-word matches beat prefixes ("roti" over "rotisserie").
                    score += 2.0 * lower.iter().filter(|w| name_words.contains(&w.as_str())).count() as f64;
                    score -= name.len() as f64 / 60.0;
                    score += f.known as f64 / 40.0;
                    hits.push((score, f));
                }
            }
        }
        hits.sort_by(|a, b| b.0.total_cmp(&a.0));
        hits.into_iter().take(limit.clamp(1, 100)).map(|(_, f)| f).collect()
    }

    /// Look a food up by its reference ("usda:167512").
    pub fn get(&self, reference: &str) -> Option<DbFood> {
        let (source, sid) = reference.split_once(':')?;
        for p in &self.parts {
            let id: Option<i64> = p
                .conn
                .query_row("SELECT id FROM food WHERE source = ?1 AND source_id = ?2", params![source, sid], |r| r.get(0))
                .optional()
                .ok()
                .flatten();
            if let Some(id) = id {
                return load(&p.conn, id);
            }
        }
        None
    }

    /// Replace one source's foods (in the writable file) in one transaction.
    pub fn replace_source(&self, info: &SourceInfo, foods: &[NewFood]) -> Result<usize> {
        let c = self.writable()?;
        c.execute_batch("BEGIN").map_err(err)?;
        let r = (|| {
            c.execute("DELETE FROM portion WHERE food_id IN (SELECT id FROM food WHERE source = ?1)", [&info.id]).map_err(err)?;
            c.execute("DELETE FROM food WHERE source = ?1", [&info.id]).map_err(err)?;
            let mut ins = c
                .prepare("INSERT INTO food (source, source_id, name, category, kcal, prot, nut) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                .map_err(err)?;
            let mut por = c.prepare("INSERT INTO portion (food_id, label, grams) VALUES (?1, ?2, ?3)").map_err(err)?;
            let mut n = 0;
            for f in foods {
                if f.name.trim().is_empty() || !f.kcal100.is_finite() {
                    continue;
                }
                let nut = serde_json::to_string(&nutrients::clean(&f.nut100)).map_err(err)?;
                let r1 = |x: f64| (x * 10.0).round() / 10.0;
                ins.execute(params![info.id, f.source_id, f.name.trim(), f.category, r1(f.kcal100), r1(f.prot100), nut]).map_err(err)?;
                let id = c.last_insert_rowid();
                for p in &f.portions {
                    if p.grams > 0.0 && p.grams.is_finite() {
                        por.execute(params![id, p.label, p.grams]).map_err(err)?;
                    }
                }
                n += 1;
            }
            c.execute(
                "INSERT INTO source (id, name, version, licence, attribution, url, imported_at, foods) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, version=excluded.version, licence=excluded.licence,
                 attribution=excluded.attribution, url=excluded.url, imported_at=excluded.imported_at, foods=excluded.foods",
                params![info.id, info.name, info.version, info.licence, info.attribution, info.url, crate::dates::now_ms(), n as i64],
            )
            .map_err(err)?;
            c.execute_batch("INSERT INTO food_fts(food_fts) VALUES('rebuild')").map_err(err)?;
            Ok(n)
        })();
        match r {
            Ok(n) => {
                c.execute_batch("COMMIT").map_err(err)?;
                Ok(n)
            }
            Err(e) => {
                let _ = c.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// Remove a downloaded source.
    pub fn remove_source(&self, id: &str) -> Result<()> {
        let c = self.writable()?;
        c.execute("DELETE FROM portion WHERE food_id IN (SELECT id FROM food WHERE source = ?1)", [id]).map_err(err)?;
        c.execute("DELETE FROM food WHERE source = ?1", [id]).map_err(err)?;
        c.execute("DELETE FROM source WHERE id = ?1", [id]).map_err(err)?;
        c.execute_batch("INSERT INTO food_fts(food_fts) VALUES('rebuild'); VACUUM;").map_err(err)?;
        Ok(())
    }
}

/// Compact a database file (after building it).
pub fn vacuum(path: &Path) -> Result<()> {
    let c = Connection::open(path).map_err(err)?;
    c.execute_batch("INSERT INTO food_fts(food_fts) VALUES('optimize'); VACUUM;").map_err(err)
}

/// Spelling variants across US, UK, Swiss and Indian names.
const SYNONYMS: &[&[&str]] = &[
    &["dal", "dahl", "dhal", "daal"],
    &["chapati", "chapatti", "chapatis", "roti"],
    &["yogurt", "yoghurt"],
    &["eggplant", "aubergine", "brinjal"],
    &["zucchini", "courgette"],
    &["cilantro", "coriander"],
    &["shrimp", "prawn", "prawns"],
    &["chickpea", "chickpeas", "garbanzo", "chana"],
    &["chili", "chilli", "chile"],
    &["okra", "bhindi"],
    &["spinach", "palak", "saag"],
    &["cottage", "paneer"],
];

fn synonyms(w: &str) -> Vec<&'static str> {
    SYNONYMS.iter().find(|g| g.contains(&w)).map(|g| g.iter().copied().filter(|x| *x != w).collect()).unwrap_or_default()
}

/// A food to import.
#[derive(Debug, Clone, Default)]
pub struct NewFood {
    pub source_id: String,
    pub name: String,
    pub category: String,
    pub kcal100: f64,
    pub prot100: f64,
    /// Everything else, including fiber.
    pub nut100: Nut,
    pub portions: Vec<Portion>,
}

fn load(c: &Connection, id: i64) -> Option<DbFood> {
    let (source, sid, name, category, kcal, prot, nut): (String, String, String, Option<String>, Option<f64>, Option<f64>, String) = c
        .query_row("SELECT source, source_id, name, category, kcal, prot, nut FROM food WHERE id = ?1", [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
        })
        .ok()?;
    let nut100: Nut = serde_json::from_str(&nut).unwrap_or_default();
    let source_name: String =
        c.query_row("SELECT name FROM source WHERE id = ?1", [&source], |r| r.get(0)).unwrap_or_else(|_| source.clone());
    let mut st = c.prepare_cached("SELECT label, grams FROM portion WHERE food_id = ?1 ORDER BY grams").ok()?;
    let portions: Vec<Portion> =
        st.query_map([id], |r| Ok(Portion { label: r.get(0)?, grams: r.get(1)? })).ok()?.flatten().take(12).collect();
    Some(DbFood {
        reference: format!("{source}:{sid}"),
        name,
        source: source.clone(),
        source_name,
        category: category.unwrap_or_default(),
        kcal100: kcal.unwrap_or(0.0),
        prot100: prot.unwrap_or(0.0),
        fiber100: nut100.get("fiber").copied(),
        known: nut100.len(),
        nut100,
        portions,
    })
}

#[doc(hidden)]
pub fn sample_db() -> FoodDb {
    let db = FoodDb::create(None).unwrap();
    let n = |pairs: &[(&str, f64)]| pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect::<Nut>();
    let foods = vec![
        NewFood {
            source_id: "1".into(),
            name: "Chicken, broilers or fryers, breast, meat only, cooked, roasted".into(),
            category: "Poultry Products".into(),
            kcal100: 165.0,
            prot100: 31.0,
            nut100: n(&[("fat", 3.6), ("sodium", 74.0), ("b12", 0.34), ("fiber", 0.0)]),
            portions: vec![Portion { label: "1 breast, bone and skin removed".into(), grams: 172.0 }],
        },
        NewFood {
            source_id: "2".into(),
            name: "Lentils, mature seeds, cooked, boiled".into(),
            category: "Legumes".into(),
            kcal100: 116.0,
            prot100: 9.0,
            nut100: n(&[("fiber", 7.9), ("iron", 3.33), ("folate", 181.0)]),
            portions: vec![Portion { label: "1 cup".into(), grams: 198.0 }],
        },
        NewFood {
            source_id: "3".into(),
            name: "Chickpeas (garbanzo beans), canned, drained".into(),
            category: "Legumes".into(),
            kcal100: 139.0,
            prot100: 7.0,
            nut100: n(&[("fiber", 6.4)]),
            portions: vec![],
        },
    ];
    let info = SourceInfo {
        id: "usda".into(),
        name: "USDA FoodData Central".into(),
        version: "test".into(),
        licence: "CC0".into(),
        attribution: "U.S. Department of Agriculture".into(),
        url: "https://fdc.nal.usda.gov".into(),
        imported_at: 0,
        foods: 0,
    };
    db.replace_source(&info, &foods).unwrap();
    db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_and_get() {
        let db = sample_db();
        assert_eq!(db.search("garbanzo", 5)[0].reference, "usda:3", "synonyms: garbanzo finds chickpeas");
        let hits = db.search("chick bre", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].reference, "usda:1");
        assert_eq!(hits[0].portions[0].grams, 172.0);
        let hits = db.search("chick", 10);
        assert_eq!(hits.len(), 2);
        let hits = db.search("chicken", 10);
        assert_eq!(hits[0].reference, "usda:1", "a whole-word match beats a prefix match");
        assert!(db.search("  ", 10).is_empty());
        assert!(db.search("\"; DROP TABLE food; --", 10).is_empty(), "query text is never SQL");
        let f = db.get("usda:2").unwrap();
        assert_eq!((f.fiber100, f.known), (Some(7.9), 3));
        assert!(db.get("usda:999").is_none());
        assert!(db.get("nonsense").is_none());
        let s = db.sources();
        assert_eq!((s[0].id.as_str(), s[0].foods), ("usda", 3));
    }

    #[test]
    fn replace_and_remove_source() {
        let db = sample_db();
        let mut info = db.sources()[0].clone();
        info.id = "swiss".into();
        info.name = "Swiss".into();
        let foods = vec![NewFood {
            source_id: "A1".into(),
            name: "Rösti".into(),
            category: "Kartoffeln".into(),
            kcal100: 150.0,
            prot100: 2.0,
            ..Default::default()
        }];
        assert_eq!(db.replace_source(&info, &foods).unwrap(), 1);
        assert_eq!(db.search("rosti", 5)[0].reference, "swiss:A1", "diacritics ignored");
        assert_eq!(db.replace_source(&info, &foods).unwrap(), 1, "re-import replaces, no duplicates");
        assert_eq!(db.search("rosti", 5).len(), 1);
        db.remove_source("swiss").unwrap();
        assert!(db.search("rosti", 5).is_empty());
        assert_eq!(db.sources().len(), 1);
    }
}
