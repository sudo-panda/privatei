//! Import/export in the `privatei-export` format (an older format name is also accepted):
//! `{"format":…,"version":1,"exported":"…","collections":{"<table>":{"<id>":{fields…}}}}`

use crate::model::*;
use crate::store::{Doc, Store, TABLES};
use serde_json::{json, Map, Value};

pub const FORMAT: &str = "privatei-export";
pub const LEGACY_FORMAT: &str = "larder-export";

/// ISO-8601 UTC timestamp for ms since the epoch.
pub fn iso_utc(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (y, m, d) = crate::dates::civil_from_days(days);
    let s = rem / 1000;
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z", s / 3600, s / 60 % 60, s % 60, rem % 1000)
}

fn collection<T: Doc>(store: &Store) -> Value {
    let mut m = Map::new();
    for (id, doc) in store.all::<T>() {
        if let Ok(v) = serde_json::to_value(doc) {
            m.insert(id.clone(), v);
        }
    }
    Value::Object(m)
}

/// Export every live row of every synced table. Never includes `local` (the AI keys).
pub fn export(store: &Store, now_ms: i64) -> Value {
    let mut c = Map::new();
    c.insert("log".into(), collection::<Log>(store));
    c.insert("plan".into(), collection::<Plan>(store));
    c.insert("stock".into(), collection::<Stock>(store));
    c.insert("list".into(), collection::<ListItem>(store));
    c.insert("meds".into(), collection::<Med>(store));
    c.insert("medlog".into(), collection::<MedLog>(store));
    c.insert("workouts".into(), collection::<Workout>(store));
    c.insert("foods".into(), collection::<Food>(store));
    c.insert("recipes".into(), collection::<Recipe>(store));
    c.insert("rules".into(), collection::<Rules>(store));
    c.insert("settings".into(), collection::<Settings>(store));
    debug_assert_eq!(c.len(), TABLES.len());
    json!({ "format": FORMAT, "version": 1, "exported": iso_utc(now_ms), "collections": c })
}

fn import_coll<T: Doc>(store: &mut Store, docs: &Map<String, Value>, fixed_id: Option<&str>) -> Result<usize, String> {
    let mut n = 0;
    for (id, doc) in docs {
        let Value::Object(obj) = doc else { continue };
        if obj.get("deleted").and_then(Value::as_bool) == Some(true) || obj.get("_deleted").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let id = fixed_id.unwrap_or(id);
        if id.is_empty() || id.len() > 200 {
            continue;
        }
        let Ok(rec) = serde_json::from_value::<T>(doc.clone()) else { continue };
        store.put(id, rec)?;
        n += 1;
    }
    Ok(n)
}

/// Import a backup. Validates the format, skips unknown collections, non-object docs
/// and `_local`/`local`. Returns how many records were imported.
pub fn import(store: &mut Store, text: &str) -> Result<usize, String> {
    let v: Value = serde_json::from_str(text).map_err(|_| "That file isn't a Private-I backup (not valid JSON).".to_string())?;
    let format = v.get("format").and_then(Value::as_str).unwrap_or("");
    if format != FORMAT && format != LEGACY_FORMAT {
        return Err("That file isn't a Private-I backup.".into());
    }
    match v.get("version").and_then(Value::as_i64) {
        Some(1) => {}
        Some(x) if x > 1 => return Err(format!("This backup is from a newer version (v{x}).")),
        _ => return Err("The backup has no valid version.".into()),
    }
    let Some(colls) = v.get("collections").and_then(Value::as_object) else {
        return Err("The backup has no collections.".into());
    };
    store.batch(|s| {
        let mut n = 0;
        for (name, docs) in colls {
            let Value::Object(docs) = docs else { continue };
            n += match name.as_str() {
                "log" => import_coll::<Log>(s, docs, None)?,
                "plan" => import_coll::<Plan>(s, docs, None)?,
                "stock" => import_coll::<Stock>(s, docs, None)?,
                "list" => import_coll::<ListItem>(s, docs, None)?,
                "meds" => import_coll::<Med>(s, docs, None)?,
                "medlog" => import_coll::<MedLog>(s, docs, None)?,
                "workouts" => import_coll::<Workout>(s, docs, None)?,
                "foods" => import_coll::<Food>(s, docs, None)?,
                "recipes" => import_coll::<Recipe>(s, docs, None)?,
                "rules" => import_coll::<Rules>(s, docs, Some("house"))?,
                "settings" => {
                    let ids: Vec<String> = s.all::<Settings>().map(|(k, _)| k.clone()).filter(|k| k != "main").collect();
                    for id in ids {
                        s.delete::<Settings>(&id)?;
                    }
                    import_coll::<Settings>(s, docs, Some("main"))?
                }
                _ => 0, // unknown, `local`, `_local`: skipped
            };
        }
        Ok(n)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_utc(1_700_000_000_123), "2023-11-14T22:13:20.123Z");
    }

    #[test]
    fn rejects_bad_files() {
        let mut s = Store::open_in_memory("d").unwrap();
        assert!(import(&mut s, "not json").is_err());
        assert!(import(&mut s, r#"{"format":"other","version":1,"collections":{}}"#).is_err());
        assert!(import(&mut s, r#"{"format":"larder-export","version":2,"collections":{}}"#).is_err());
        assert!(import(&mut s, r#"{"format":"larder-export","collections":{}}"#).is_err());
        assert!(import(&mut s, r#"{"format":"larder-export","version":1}"#).is_err());
        assert_eq!(s.all::<Log>().count(), 0);
    }

    #[test]
    fn skips_unknown_and_local() {
        let mut s = Store::open_in_memory("d").unwrap();
        let f = r#"{"format":"larder-export","version":1,"exported":"x","collections":{
            "log":{"a":{"date":"2026-11-01","meal":"Lunch","name":"Egg","kcal":79,"prot":7,"est":false,"ts":1},"b":"nope","c":[1]},
            "local":{"ai":{"geminiKey":"SECRET"}},"_local":{"ai":{"geminiKey":"SECRET"}},
            "weird":{"x":{"a":1}},
            "settings":{"whatever":{"weightKg":80,"kcalTarget":1600,"protFloor":100,"addBurn":true}},
            "rules":{"house":{"text":"no oil"}}}}"#;
        assert_eq!(import(&mut s, f).unwrap(), 3);
        assert_eq!(s.settings().weight_kg, 80.0);
        assert!(s.settings().add_burn);
        assert_eq!(s.rules_text(), "no oil");
        assert_eq!(s.ai_settings().gemini_key, "");
    }

    #[test]
    fn export_without_keys() {
        let mut s = Store::open_in_memory("d").unwrap();
        s.put("ai", AiSettings { gemini_key: "SECRET-KEY".into(), ..Default::default() }).unwrap();
        s.put("x", Log { name: "Egg".into(), kcal: 79.0, ..Default::default() }).unwrap();
        s.put("y", Log { name: "Gone".into(), ..Default::default() }).unwrap();
        s.delete::<Log>("y").unwrap();
        let v = export(&s, 0);
        let text = v.to_string();
        assert!(!text.contains("SECRET"));
        assert!(!text.contains("geminiKey"));
        assert!(!text.contains("Gone"));
        assert_eq!(v["format"], FORMAT);
        assert_eq!(v["collections"]["log"]["x"]["name"], "Egg");
        assert!(v["collections"].get("local").is_none());
        // And it imports back.
        let mut s2 = Store::open_in_memory("d").unwrap();
        assert_eq!(import(&mut s2, &text).unwrap(), 1);
    }
}
