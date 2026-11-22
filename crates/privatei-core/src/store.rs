//! SQLite storage with an in-memory cache.
//!
//! Every table has `id TEXT PRIMARY KEY, updated_at INTEGER (ms), deleted INTEGER,
//! device TEXT` plus the record as JSON in `data`. Deletes are tombstones so the
//! data is sync-ready: [`Store::changes_since`] returns every changed row.

use crate::model::*;
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 2;

/// Synced tables, in export order. `local` is never synced or exported.
pub const TABLES: [&str; 11] = ["log", "plan", "stock", "list", "meds", "medlog", "workouts", "foods", "recipes", "rules", "settings"];

#[derive(Debug, Clone)]
pub struct Rec<T> {
    pub updated_at: i64,
    pub deleted: bool,
    pub device: String,
    pub data: T,
}

pub type Table<T> = BTreeMap<String, Rec<T>>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub table: String,
    pub id: String,
    pub updated_at: i64,
    pub deleted: bool,
    pub device: String,
    pub data: serde_json::Value,
}

pub trait Doc: Serialize + DeserializeOwned + Clone + Default + 'static {
    const TABLE: &'static str;
    fn table(s: &Store) -> &Table<Self>;
    fn table_mut(s: &mut Store) -> &mut Table<Self>;
}

pub struct Store {
    conn: Connection,
    device: String,
    last_ts: i64,
    /// Rows that could not be read at startup (corrupt JSON).
    pub skipped: usize,
    pub log: Table<Log>,
    pub plan: Table<Plan>,
    pub stock: Table<Stock>,
    pub list: Table<ListItem>,
    pub meds: Table<Med>,
    pub medlog: Table<MedLog>,
    pub workouts: Table<Workout>,
    pub foods: Table<Food>,
    pub rules: Table<Rules>,
    pub settings: Table<Settings>,
    pub recipes: Table<Recipe>,
    pub local: Table<AiSettings>,
}

macro_rules! doc {
    ($t:ty, $name:literal, $field:ident) => {
        impl Doc for $t {
            const TABLE: &'static str = $name;
            fn table(s: &Store) -> &Table<Self> {
                &s.$field
            }
            fn table_mut(s: &mut Store) -> &mut Table<Self> {
                &mut s.$field
            }
        }
    };
}
doc!(Log, "log", log);
doc!(Plan, "plan", plan);
doc!(Stock, "stock", stock);
doc!(ListItem, "list", list);
doc!(Med, "meds", meds);
doc!(MedLog, "medlog", medlog);
doc!(Workout, "workouts", workouts);
doc!(Food, "foods", foods);
doc!(Rules, "rules", rules);
doc!(Settings, "settings", settings);
doc!(Recipe, "recipes", recipes);
doc!(AiSettings, "local", local);

pub type Result<T> = std::result::Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    format!("Storage error: {e}")
}

impl Store {
    pub fn open(path: &Path, device: &str) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        Self::init(Connection::open(path).map_err(err)?, device)
    }

    pub fn open_in_memory(device: &str) -> Result<Store> {
        Self::init(Connection::open_in_memory().map_err(err)?, device)
    }

    fn init(conn: Connection, device: &str) -> Result<Store> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;").map_err(err)?;
        migrate(&conn)?;
        let mut s = Store {
            conn,
            device: device.to_string(),
            last_ts: 0,
            skipped: 0,
            log: Table::new(),
            plan: Table::new(),
            stock: Table::new(),
            list: Table::new(),
            meds: Table::new(),
            medlog: Table::new(),
            workouts: Table::new(),
            foods: Table::new(),
            rules: Table::new(),
            settings: Table::new(),
            recipes: Table::new(),
            local: Table::new(),
        };
        s.load::<Log>()?;
        s.load::<Plan>()?;
        s.load::<Stock>()?;
        s.load::<ListItem>()?;
        s.load::<Med>()?;
        s.load::<MedLog>()?;
        s.load::<Workout>()?;
        s.load::<Food>()?;
        s.load::<Rules>()?;
        s.load::<Settings>()?;
        s.load::<Recipe>()?;
        s.load::<AiSettings>()?;
        Ok(s)
    }

    fn load<T: Doc>(&mut self) -> Result<()> {
        let sql = format!("SELECT id, updated_at, deleted, device, data FROM \"{}\"", T::TABLE);
        let mut stmt = self.conn.prepare(&sql).map_err(err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(err)?;
        let mut out = Table::new();
        let mut skipped = 0;
        let mut last = 0;
        for row in rows {
            let Ok((id, updated_at, deleted, device, data)) = row else {
                skipped += 1;
                continue;
            };
            let parsed = data.as_deref().and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok());
            let Some(data) = parsed.filter(|v| v.is_object()).and_then(|v| serde_json::from_value::<T>(v).ok()) else {
                skipped += 1;
                continue;
            };
            last = last.max(updated_at);
            out.insert(id, Rec { updated_at, deleted: deleted != 0, device: device.unwrap_or_default(), data });
        }
        drop(stmt);
        *T::table_mut(self) = out;
        self.skipped += skipped;
        self.last_ts = self.last_ts.max(last);
        Ok(())
    }

    /// A strictly increasing timestamp, so rapid writes keep their order.
    fn tick(&mut self) -> i64 {
        let now = crate::dates::now_ms().max(self.last_ts + 1);
        self.last_ts = now;
        now
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    /// Insert or replace a record.
    pub fn put<T: Doc>(&mut self, id: &str, data: T) -> Result<()> {
        let ts = self.tick();
        self.write::<T>(id, ts, false, &data)?;
        let device = self.device.clone();
        T::table_mut(self).insert(id.to_string(), Rec { updated_at: ts, deleted: false, device, data });
        Ok(())
    }

    /// Tombstone a record. Returns false if it did not exist.
    pub fn delete<T: Doc>(&mut self, id: &str) -> Result<bool> {
        let Some(data) = T::table(self).get(id).filter(|r| !r.deleted).map(|r| r.data.clone()) else {
            return Ok(false);
        };
        let ts = self.tick();
        self.write::<T>(id, ts, true, &data)?;
        let device = self.device.clone();
        T::table_mut(self).insert(id.to_string(), Rec { updated_at: ts, deleted: true, device, data });
        Ok(true)
    }

    fn write<T: Doc>(&self, id: &str, ts: i64, deleted: bool, data: &T) -> Result<()> {
        let json = serde_json::to_string(data).map_err(err)?;
        let sql = format!(
            "INSERT INTO \"{}\" (id, updated_at, deleted, device, data) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET updated_at=excluded.updated_at, deleted=excluded.deleted,
             device=excluded.device, data=excluded.data",
            T::TABLE
        );
        self.conn.execute(&sql, params![id, ts, deleted as i64, self.device, json]).map_err(err)?;
        Ok(())
    }

    pub fn get<T: Doc>(&self, id: &str) -> Option<&T> {
        T::table(self).get(id).filter(|r| !r.deleted).map(|r| &r.data)
    }

    /// Every live (non-deleted) record.
    pub fn all<T: Doc + 'static>(&self) -> impl Iterator<Item = (&String, &T)> + '_ {
        T::table(self).iter().filter(|(_, r)| !r.deleted).map(|(k, r)| (k, &r.data))
    }

    /// Run several writes in one transaction (much faster for imports).
    pub fn batch<R>(&mut self, f: impl FnOnce(&mut Store) -> Result<R>) -> Result<R> {
        self.conn.execute_batch("BEGIN").map_err(err)?;
        match f(self) {
            Ok(r) => {
                self.conn.execute_batch("COMMIT").map_err(err)?;
                Ok(r)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                // Reload so memory matches disk again.
                let _ = self.reload();
                Err(e)
            }
        }
    }

    fn reload(&mut self) -> Result<()> {
        self.skipped = 0;
        self.load::<Log>()?;
        self.load::<Plan>()?;
        self.load::<Stock>()?;
        self.load::<ListItem>()?;
        self.load::<Med>()?;
        self.load::<MedLog>()?;
        self.load::<Workout>()?;
        self.load::<Food>()?;
        self.load::<Rules>()?;
        self.load::<Settings>()?;
        self.load::<Recipe>()?;
        self.load::<AiSettings>()
    }

    /// Every synced row changed after `ts`, tombstones included. Never includes `local`.
    pub fn changes_since(&self, ts: i64) -> Vec<Change> {
        let mut out = vec![];
        fn add<T: Doc>(s: &Store, ts: i64, out: &mut Vec<Change>) {
            for (id, r) in T::table(s) {
                if r.updated_at > ts {
                    out.push(Change {
                        table: T::TABLE.to_string(),
                        id: id.clone(),
                        updated_at: r.updated_at,
                        deleted: r.deleted,
                        device: r.device.clone(),
                        data: serde_json::to_value(&r.data).unwrap_or_default(),
                    });
                }
            }
        }
        add::<Log>(self, ts, &mut out);
        add::<Plan>(self, ts, &mut out);
        add::<Stock>(self, ts, &mut out);
        add::<ListItem>(self, ts, &mut out);
        add::<Med>(self, ts, &mut out);
        add::<MedLog>(self, ts, &mut out);
        add::<Workout>(self, ts, &mut out);
        add::<Food>(self, ts, &mut out);
        add::<Rules>(self, ts, &mut out);
        add::<Settings>(self, ts, &mut out);
        add::<Recipe>(self, ts, &mut out);
        out.sort_by_key(|c| c.updated_at);
        out
    }

    pub fn settings(&self) -> Settings {
        self.all::<Settings>().next().map(|(_, s)| s.clone()).unwrap_or_default()
    }

    pub fn rules_text(&self) -> String {
        self.get::<Rules>("house").map(|r| r.text.clone()).unwrap_or_default()
    }

    pub fn ai_settings(&self) -> AiSettings {
        self.get::<AiSettings>("ai").cloned().unwrap_or_default()
    }

    /// Raw connection for tests that need to plant bad rows.
    #[doc(hidden)]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn create_table(t: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS \"{t}\" (id TEXT PRIMARY KEY, updated_at INTEGER NOT NULL,
         deleted INTEGER NOT NULL DEFAULT 0, device TEXT, data TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS \"{t}_updated\" ON \"{t}\"(updated_at);"
    )
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(err)?;
    if version < 1 {
        let mut sql = String::from("BEGIN;");
        for t in TABLES.iter().chain(std::iter::once(&"local")) {
            sql.push_str(&create_table(t));
        }
        sql.push_str("PRAGMA user_version = 2; COMMIT;");
        conn.execute_batch(&sql).map_err(err)?;
        return Ok(());
    }
    if version < 2 {
        // v2: recipes
        conn.execute_batch(&format!("BEGIN; {} PRAGMA user_version = 2; COMMIT;", create_table("recipes"))).map_err(err)?;
    }
    if version > SCHEMA_VERSION {
        return Err(format!("This database was made by a newer Private-I (schema {version})."));
    }
    Ok(())
}

/// A short unique id: time + counter + randomness.
pub fn new_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u32(c);
    h.write_i64(crate::dates::now_ms());
    format!("{:x}{:04x}{:06x}", crate::dates::now_ms(), c & 0xffff, h.finish() & 0xff_ffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(name: &str, kcal: f64) -> Log {
        Log { date: "2026-11-22".into(), meal: "Lunch".into(), name: name.into(), kcal, ..Default::default() }
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("privatei-test-{}-{}", name, new_id()));
        std::fs::create_dir_all(&p).unwrap();
        p.join("data.sqlite")
    }

    #[test]
    fn round_trip_survives_reopen() {
        let path = tmp("reopen");
        {
            let mut s = Store::open(&path, "dev1").unwrap();
            s.put("a", log("Egg", 79.0)).unwrap();
            s.put("b", log("Toast", 98.0)).unwrap();
            s.delete::<Log>("b").unwrap();
            s.put("h", Rules { text: "no oil".into() }).unwrap();
        }
        let s = Store::open(&path, "dev1").unwrap();
        assert_eq!(s.get::<Log>("a").unwrap().name, "Egg");
        assert!(s.get::<Log>("b").is_none());
        assert_eq!(s.all::<Log>().count(), 1);
        assert!(s.log["b"].deleted, "tombstone kept");
        assert_eq!(s.get::<Rules>("h").unwrap().text, "no oil");
    }

    #[test]
    fn tombstones_in_changes() {
        let mut s = Store::open_in_memory("d").unwrap();
        s.put("a", log("Egg", 79.0)).unwrap();
        let t0 = s.changes_since(0)[0].updated_at;
        s.delete::<Log>("a").unwrap();
        let ch = s.changes_since(t0);
        assert_eq!(ch.len(), 1);
        assert!(ch[0].deleted);
        assert_eq!(ch[0].table, "log");
        assert!(!s.delete::<Log>("a").unwrap(), "already deleted");
        s.put("ai", AiSettings { gemini_key: "k".into(), ..Default::default() }).unwrap();
        assert!(s.changes_since(0).iter().all(|c| c.table != "local"), "local never syncs");
    }

    #[test]
    fn rapid_writes_keep_last_value() {
        let path = tmp("rapid");
        {
            let mut s = Store::open(&path, "d").unwrap();
            for i in 0..200 {
                s.put("x", log("Egg", i as f64)).unwrap();
            }
            assert_eq!(s.get::<Log>("x").unwrap().kcal, 199.0);
        }
        let s = Store::open(&path, "d").unwrap();
        assert_eq!(s.get::<Log>("x").unwrap().kcal, 199.0);
        assert_eq!(s.all::<Log>().count(), 1);
    }

    #[test]
    fn corrupt_row_skipped_at_startup() {
        let path = tmp("corrupt");
        {
            let mut s = Store::open(&path, "d").unwrap();
            s.put("good", log("Egg", 79.0)).unwrap();
            s.conn().execute("INSERT INTO log (id, updated_at, deleted, device, data) VALUES ('bad', 1, 0, 'd', '{not json')", []).unwrap();
            s.conn().execute("INSERT INTO log (id, updated_at, deleted, device, data) VALUES ('arr', 1, 0, 'd', '[1,2]')", []).unwrap();
        }
        let s = Store::open(&path, "d").unwrap();
        assert_eq!(s.skipped, 2);
        assert_eq!(s.all::<Log>().count(), 1);
        assert!(s.get::<Log>("good").is_some());
    }

    #[test]
    fn migrates_v1_to_v2() {
        let path = tmp("migrate");
        {
            // A version-1 database without the recipes table.
            let c = Connection::open(&path).unwrap();
            for t in ["log", "plan", "stock", "list", "meds", "medlog", "workouts", "foods", "rules", "settings", "local"] {
                c.execute_batch(&create_table(t)).unwrap();
            }
            c.execute_batch("PRAGMA user_version = 1;").unwrap();
            c.execute(
                "INSERT INTO log (id, updated_at, deleted, device, data) VALUES ('a', 1, 0, 'd', '{\"name\":\"Egg\",\"kcal\":79}')",
                [],
            )
            .unwrap();
        }
        let mut s = Store::open(&path, "d").unwrap();
        assert_eq!(s.get::<Log>("a").unwrap().name, "Egg", "old data kept");
        s.put("r", Recipe { name: "Dal".into(), servings: 4.0, ..Default::default() }).unwrap();
        let v: i64 = s.conn().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 2);
        drop(s);
        assert_eq!(Store::open(&path, "d").unwrap().get::<Recipe>("r").unwrap().name, "Dal");
        // A database from a newer app is refused rather than damaged.
        Connection::open(&path).unwrap().execute_batch("PRAGMA user_version = 99;").unwrap();
        assert!(Store::open(&path, "d").is_err());
    }

    #[test]
    fn ids_unique() {
        let ids: std::collections::HashSet<_> = (0..1000).map(|_| new_id()).collect();
        assert_eq!(ids.len(), 1000);
    }
}
