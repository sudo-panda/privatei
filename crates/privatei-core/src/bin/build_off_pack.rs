//! Build the Swiss Open Food Facts pack from OFF's worldwide CSV export.
//!
//! curl -L https://static.openfoodfacts.org/data/en.openfoodfacts.org.products.csv.gz |
//!   cargo run --release -p privatei-core --features import --bin build-off-pack -- \
//!   --date 2026-11-23 --out off-ch.jsonl.gz
//!
//! Reads the gzipped CSV from stdin (or `--in <file.csv.gz>`), writes the gzipped pack.
//! The data is ODbL: publish the pack with the "Open Food Facts contributors" credit.

use flate2::{read::MultiGzDecoder, write::GzEncoder, Compression};
use std::io::{BufReader, BufWriter, Read};

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut input, mut out, mut date) = (None::<String>, String::from("off-ch.jsonl.gz"), String::new());
    while let Some(a) = args.next() {
        match a.as_str() {
            "--in" => input = Some(args.next().expect("--in needs a file")),
            "--out" => out = args.next().expect("--out needs a file"),
            "--date" => date = args.next().expect("--date needs YYYY-MM-DD"),
            other => panic!("unknown argument {other}"),
        }
    }
    assert!(date.len() == 10, "--date YYYY-MM-DD is required");
    let src: Box<dyn Read> = match input {
        Some(p) => Box::new(std::fs::File::open(p).expect("open input")),
        None => Box::new(std::io::stdin().lock()),
    };
    let csv = MultiGzDecoder::new(BufReader::with_capacity(1 << 20, src));
    let file = std::fs::File::create(&out).expect("create output");
    let mut gz = GzEncoder::new(BufWriter::new(file), Compression::best());
    let n = privatei_core::off::csv_to_pack(csv, &mut gz, &date).expect("build pack");
    gz.finish().expect("finish gzip");
    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    eprintln!("Wrote {out}: {n} products, {} KB", size / 1024);
    if n < 1000 {
        eprintln!("Too few products; the export may have changed.");
        std::process::exit(1);
    }
}
