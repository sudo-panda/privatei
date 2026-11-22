//! Build the food database shipped inside the app.
//!
//! cargo run -p privatei-core --features import --bin build-fooddb -- \
//!   --out apps/desktop/resources/foods.sqlite --version 2026-10 <usda-csv-dir>...
//!
//! Optional: --swiss <file.xlsx> (only for local testing; the Swiss database is
//! downloaded by the app, not shipped).

use privatei_core::fooddb::FoodDb;
use privatei_core::importers;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut out = PathBuf::from("foods.sqlite");
    let mut version = String::from("unknown");
    let mut swiss: Option<PathBuf> = None;
    let mut dirs: Vec<PathBuf> = vec![];
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().expect("--out needs a path").into(),
            "--version" => version = args.next().expect("--version needs a value"),
            "--swiss" => swiss = Some(args.next().expect("--swiss needs a file").into()),
            _ => dirs.push(a.into()),
        }
    }
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p).expect("create output folder");
    }
    let db = FoodDb::create(Some(&out)).expect("create database");
    if !dirs.is_empty() {
        let refs: Vec<&std::path::Path> = dirs.iter().map(|d| d.as_path()).collect();
        let foods = importers::usda_from_dirs(&refs).expect("read USDA");
        let n = db.replace_source(&importers::usda_info(&version), &foods).expect("write USDA");
        eprintln!("USDA: {n} foods");
    }
    if let Some(s) = swiss {
        let bytes = std::fs::read(&s).expect("read Swiss xlsx");
        let foods = importers::swiss_from_xlsx(&bytes).expect("parse Swiss");
        let n = db.replace_source(&importers::swiss_info("local"), &foods).expect("write Swiss");
        eprintln!("Swiss: {n} foods");
    }
    drop(db);
    let c = rusqlite_vacuum(&out);
    eprintln!("Wrote {} ({} KB)", out.display(), c / 1024);
}

fn rusqlite_vacuum(p: &std::path::Path) -> u64 {
    // Re-open through the core to VACUUM; keeps the bin free of extra deps.
    let _ = privatei_core::fooddb::vacuum(p);
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}
