//! The food table, food matching, amount parsing and the local estimate.

use crate::model::{meal_for_time, Food};
use crate::nutrients::{self, Nut};
use serde::{Deserialize, Serialize};

/// (name, kcal per 100 g, protein per 100 g)
pub const TABLE: &[(&str, f64, f64)] = &[
    ("chicken thigh roasted skin-off", 185.0, 24.0),
    ("chicken breast", 165.0, 31.0),
    ("cooked prawns", 99.0, 21.0),
    ("egg", 143.0, 12.6),
    ("cottage cheese", 98.0, 11.0),
    ("skyr", 63.0, 11.0),
    ("smoked tofu", 190.0, 19.0),
    ("firm tofu roasted dry", 190.0, 21.0),
    ("soy granulate dry", 345.0, 52.0),
    ("chickpeas drained", 150.0, 8.5),
    ("kidney/white beans drained", 130.0, 8.5),
    ("tinned tomatoes", 32.0, 1.6),
    ("oats", 370.0, 13.0),
    ("ground flaxseed", 534.0, 18.0),
    ("rice cooked", 130.0, 2.7),
    ("roti", 300.0, 10.0),
    ("wholegrain bread", 244.0, 8.5),
    ("Erdnussmus", 600.0, 25.0),
    ("banana", 89.0, 1.1),
    ("apple", 52.0, 0.3),
    ("grapes", 69.0, 0.7),
    ("cucumber", 15.0, 0.7),
    ("frozen raspberries", 52.0, 1.2),
    ("frozen spinach", 25.0, 2.9),
    ("potato roasted dry", 120.0, 2.7),
    ("bell pepper roasted dry", 40.0, 1.5),
    ("onion roasted dry", 62.0, 1.7),
    ("oil", 884.0, 0.0),
    ("ghee", 900.0, 0.0),
];

/// Fiber and micronutrients per 100 g, as eaten: (name, fiber g, vitamin C mg, vitamin D µg,
/// B12 µg, iron mg, calcium mg). Approximate values from standard food tables (USDA/Swiss);
/// tofu calcium depends on the coagulant.
pub const EXTRA: &[(&str, f64, [f64; 5])] = &[
    ("chicken thigh roasted skin-off", 0.0, [0.0, 0.1, 0.4, 1.0, 10.0]),
    ("chicken breast", 0.0, [0.0, 0.1, 0.3, 0.5, 15.0]),
    ("cooked prawns", 0.0, [0.0, 0.0, 1.5, 0.3, 70.0]),
    ("egg", 0.0, [0.0, 2.0, 0.9, 1.8, 56.0]),
    ("cottage cheese", 0.0, [0.0, 0.0, 0.4, 0.1, 83.0]),
    ("skyr", 0.0, [0.0, 0.0, 0.5, 0.1, 120.0]),
    ("smoked tofu", 1.5, [0.0, 0.0, 0.0, 2.5, 200.0]),
    ("firm tofu roasted dry", 2.5, [0.0, 0.0, 0.0, 3.0, 250.0]),
    ("soy granulate dry", 17.0, [0.0, 0.0, 0.0, 9.0, 250.0]),
    ("chickpeas drained", 7.6, [1.3, 0.0, 0.0, 1.5, 40.0]),
    ("kidney/white beans drained", 6.4, [1.0, 0.0, 0.0, 2.0, 45.0]),
    ("tinned tomatoes", 1.5, [12.0, 0.0, 0.0, 1.0, 30.0]),
    ("oats", 10.1, [0.0, 0.0, 0.0, 4.3, 54.0]),
    ("ground flaxseed", 27.3, [0.6, 0.0, 0.0, 5.7, 255.0]),
    ("rice cooked", 0.4, [0.0, 0.0, 0.0, 0.2, 10.0]),
    ("roti", 4.5, [0.0, 0.0, 0.0, 2.5, 30.0]),
    ("wholegrain bread", 6.0, [0.0, 0.0, 0.0, 2.5, 100.0]),
    ("Erdnussmus", 6.0, [0.0, 0.0, 0.0, 1.9, 45.0]),
    ("banana", 2.6, [8.7, 0.0, 0.0, 0.3, 5.0]),
    ("apple", 2.4, [4.6, 0.0, 0.0, 0.1, 6.0]),
    ("grapes", 0.9, [3.2, 0.0, 0.0, 0.4, 10.0]),
    ("cucumber", 0.5, [2.8, 0.0, 0.0, 0.3, 16.0]),
    ("frozen raspberries", 6.5, [26.0, 0.0, 0.0, 0.7, 25.0]),
    ("frozen spinach", 3.0, [2.2, 0.0, 0.0, 1.9, 150.0]),
    ("potato roasted dry", 2.2, [10.0, 0.0, 0.0, 1.0, 12.0]),
    ("bell pepper roasted dry", 2.0, [100.0, 0.0, 0.0, 0.5, 9.0]),
    ("onion roasted dry", 2.4, [6.0, 0.0, 0.0, 0.3, 25.0]),
    ("oil", 0.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
    ("ghee", 0.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
];

/// Keys of the five values in each EXTRA row.
const EXTRA_KEYS: [&str; 5] = ["vitC", "vitD", "b12", "iron", "calcium"];

fn extra_for(name: &str) -> (Option<f64>, Option<Nut>) {
    match EXTRA.iter().find(|(n, _, _)| *n == name) {
        Some((_, f, m)) => (Some(*f), Some(EXTRA_KEYS.iter().zip(m.iter()).map(|(k, v)| (k.to_string(), *v)).collect())),
        None => (None, None),
    }
}

/// Foods richest in a nutrient per 100 g (for "what's missing" tips). `which`: "fiber" or a micro key.
pub fn richest(which: &str, n: usize) -> Vec<String> {
    let idx = EXTRA_KEYS.iter().position(|k| *k == which);
    let mut v: Vec<(&str, f64)> =
        EXTRA.iter().map(|(name, f, m)| (*name, idx.map(|i| m[i]).unwrap_or(*f))).filter(|(_, x)| *x > 0.0).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.into_iter().take(n).map(|(name, _)| capitalize(name)).collect()
}

/// Piece weights in grams.
pub const PIECES: &[(&str, f64)] = &[
    ("egg", 55.0),
    ("banana", 120.0),
    ("apple", 180.0),
    ("roti", 40.0),
    ("wholegrain bread", 40.0),
    ("cucumber", 300.0),
    ("bell pepper roasted dry", 160.0),
    ("onion roasted dry", 110.0),
    ("potato roasted dry", 170.0),
];

/// alias words → table name
pub const ALIASES: &[(&str, &str)] = &[
    ("peanut butter", "Erdnussmus"),
    ("pb", "Erdnussmus"),
    ("bread", "wholegrain bread"),
    ("toast", "wholegrain bread"),
    ("chapati", "roti"),
    ("shrimp", "cooked prawns"),
    ("beans", "kidney/white beans drained"),
    ("tofu", "firm tofu roasted dry"),
    ("flax", "ground flaxseed"),
    ("flaxseed", "ground flaxseed"),
    ("olive oil", "oil"),
    ("potato", "potato roasted dry"),
    ("pepper", "bell pepper roasted dry"),
    ("onion", "onion roasted dry"),
    ("rice", "rice cooked"),
    ("raspberries", "frozen raspberries"),
    ("spinach", "frozen spinach"),
    ("tomatoes", "tinned tomatoes"),
];

const MODIFIERS: &[&str] = &["roasted", "dry", "drained", "cooked", "skin-off", "tinned", "frozen", "ground", "firm"];

/// The table as text for AI prompts: `name kcal/prot; …`.
pub fn table_text() -> String {
    TABLE
        .iter()
        .map(|(n, k, p)| match extra_for(n).0 {
            Some(f) => format!("{n} {k}/{p}/{f}"),
            None => format!("{n} {k}/{p}"),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn piece_weight(table_name: &str) -> f64 {
    PIECES.iter().find(|(n, _)| *n == table_name).map(|p| p.1).unwrap_or(0.0)
}

/// Singular form: strip "es" if longer than 4 letters, else "s" if longer than 3.
pub fn singular(w: &str) -> String {
    let n = w.chars().count();
    if n > 4 && w.ends_with("es") {
        w[..w.len() - 2].to_string()
    } else if n > 3 && w.ends_with('s') {
        w[..w.len() - 1].to_string()
    } else {
        w.to_string()
    }
}

/// Words compare equal after singularising; "apple"/"appl" (from "apples") also match.
fn word_eq(a: &str, b: &str) -> bool {
    a == b
        || (a.len() + 1 == b.len() && b.starts_with(a) && b.ends_with('e'))
        || (b.len() + 1 == a.len() && a.starts_with(b) && a.ends_with('e'))
}

/// Lowercase, split and singularise.
pub fn words(text: &str) -> Vec<String> {
    text.to_lowercase().split(|c: char| !(c.is_alphanumeric() || c == '-')).filter(|w| !w.is_empty() && *w != "-").map(singular).collect()
}

fn key_words(name: &str) -> Vec<String> {
    let mods: Vec<String> = MODIFIERS.iter().map(|m| singular(m)).collect();
    words(name).into_iter().filter(|w| !mods.contains(w)).collect()
}

fn has(user: &[String], w: &str) -> bool {
    user.iter().any(|u| word_eq(u, w))
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoodRef {
    pub name: String,
    pub kcal100: f64,
    pub prot100: f64,
    pub piece: f64,
    pub learned: bool,
    pub fiber100: Option<f64>,
    pub nut100: Option<Nut>,
}

fn table_ref(name: &str) -> Option<FoodRef> {
    TABLE.iter().find(|(n, _, _)| *n == name).map(|(n, k, p)| {
        let (fiber100, nut100) = extra_for(n);
        FoodRef { name: n.to_string(), kcal100: *k, prot100: *p, piece: piece_weight(n), learned: false, fiber100, nut100 }
    })
}

/// Best match in the base table only.
pub fn match_table(user: &[String]) -> Option<FoodRef> {
    let mut best: Option<(f64, usize, &str)> = None;
    for (name, _, _) in TABLE {
        let keys = key_words(name);
        let Some(noun) = keys.last() else { continue };
        if !has(user, noun) {
            continue;
        }
        let matched = keys.iter().filter(|k| has(user, k)).count();
        let score = matched as f64 / keys.len() as f64;
        if score >= 0.5 && best.is_none_or(|(s, m, _)| score > s || (score == s && matched > m)) {
            best = Some((score, matched, name));
        }
    }
    let direct = best.and_then(|(s, _, n)| Some((s, table_ref(n)?)));
    if let Some((s, r)) = &direct {
        if *s >= 1.0 {
            return Some(r.clone());
        }
    }
    for (alias, target) in ALIASES {
        if words(alias).iter().all(|w| has(user, w)) {
            return table_ref(target);
        }
    }
    direct.map(|(_, r)| r)
}

/// Learned foods first (every key word must match), then the base table.
pub fn match_food(user: &[String], learned: &[Food]) -> Option<FoodRef> {
    let mut best: Option<(usize, &Food)> = None;
    for f in learned {
        let keys = key_words(&f.name);
        if !keys.is_empty() && keys.iter().all(|k| has(user, k)) && best.is_none_or(|(n, _)| keys.len() > n) {
            best = Some((keys.len(), f));
        }
    }
    if let Some((_, f)) = best {
        return Some(FoodRef {
            name: f.name.clone(),
            kcal100: f.kcal100,
            prot100: f.prot100,
            piece: f.piece,
            learned: true,
            fiber100: f.fiber100,
            nut100: f.nut100.clone(),
        });
    }
    match_table(user)
}

/// Whether the base table already covers a food name.
pub fn table_covers(name: &str) -> bool {
    match_table(&words(name)).is_some()
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Amount {
    /// Measured grams (g, kg, ml, tbsp, tsp).
    pub grams: Option<f64>,
    /// A piece count ("2 eggs", "2x", "two").
    pub count: Option<f64>,
    /// The remaining words describing the food.
    pub food: Vec<String>,
}

fn number_word(w: &str) -> Option<f64> {
    Some(match w {
        "a" | "an" | "one" | "ein" | "eine" | "einen" | "eins" => 1.0,
        "two" | "zwei" => 2.0,
        "three" | "drei" => 3.0,
        "four" | "vier" => 4.0,
        "half" | "halb" | "halbe" | "halber" => 0.5,
        _ => return None,
    })
}

fn unit_grams(u: &str) -> Option<f64> {
    Some(match u {
        "g" | "gr" | "gram" | "grams" | "gramm" | "ml" => 1.0,
        "kg" | "l" => 1000.0,
        "cl" => 10.0,
        "dl" => 100.0,
        "tbsp" | "el" | "tablespoon" | "tablespoons" | "essl" => 15.0,
        "tsp" | "tl" | "teaspoon" | "teaspoons" => 5.0,
        _ => return None,
    })
}

fn is_count_unit(u: &str) -> bool {
    matches!(u, "x" | "pc" | "pcs" | "piece" | "pieces" | "stück" | "stk" | "slice" | "slices" | "scheibe" | "scheiben")
}

/// Parse an amount out of a food phrase.
pub fn parse_amount(part: &str) -> Amount {
    // Separate numbers from letters: "200g" → "200 g", "2x" → "2 x".
    let mut s = String::new();
    let mut prev: Option<char> = None;
    for c in part.to_lowercase().chars() {
        if let Some(p) = prev {
            if (p.is_ascii_digit() && c.is_alphabetic()) || (p.is_alphabetic() && c.is_ascii_digit()) {
                s.push(' ');
            }
        }
        s.push(c);
        prev = Some(c);
    }
    let toks: Vec<&str> = s.split(|c: char| c.is_whitespace() || c == '(' || c == ')').filter(|t| !t.is_empty()).collect();
    let mut a = Amount::default();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        let n = t.replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0);
        if let Some(n) = n {
            let next = toks.get(i + 1).copied().unwrap_or("");
            if let Some(g) = unit_grams(next) {
                a.grams = Some(a.grams.unwrap_or(0.0) + n * g);
                i += 2;
                continue;
            }
            a.count = Some(a.count.unwrap_or(1.0) * n);
            i += if is_count_unit(next) { 2 } else { 1 };
            continue;
        }
        if let Some(n) = number_word(t) {
            a.count = Some(a.count.unwrap_or(1.0) * n);
        } else if let Some(g) = unit_grams(t).filter(|_| matches!(t, "tbsp" | "el" | "tsp" | "tl" | "tablespoon" | "teaspoon")) {
            a.grams = Some(a.grams.unwrap_or(0.0) + g);
        } else if !is_count_unit(t) && t != "x" && t != "of" {
            a.food.push(t.to_string());
        }
        i += 1;
    }
    a
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    pub grams: Option<f64>,
    pub kcal: f64,
    pub prot: f64,
    pub est: bool,
    pub meal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fiber: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nut: Option<Nut>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEstimate {
    pub items: Vec<Item>,
    pub unknown: Vec<String>,
    pub meal: String,
}

pub fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

pub fn fmt_num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        format!("{}", round1(x))
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// kcal and protein for `grams` of a food.
pub fn nutrients(f: &FoodRef, grams: f64) -> (f64, f64) {
    ((f.kcal100 * grams / 100.0).round(), round1(f.prot100 * grams / 100.0))
}

/// Fiber and micronutrients for `grams` of a food, when known.
pub fn extras(f: &FoodRef, grams: f64) -> (Option<f64>, Option<Nut>) {
    (f.fiber100.map(|x| round1(x * grams / 100.0)), f.nut100.as_ref().map(|m| nutrients::scale(m, grams / 100.0)))
}

/// Place one phrase, if possible: (food, grams, estimated?).
pub fn place(part: &str, learned: &[Food]) -> Option<(FoodRef, f64, bool)> {
    let a = parse_amount(part);
    let user: Vec<String> = a.food.iter().map(|w| singular(w)).collect();
    let f = match_food(&user, learned)?;
    let (grams, est) = match (a.grams, a.count) {
        (Some(g), _) if g > 0.0 => (g, false),
        (_, Some(n)) if f.piece > 0.0 && n > 0.0 => (n * f.piece, true),
        (None, None) if f.piece > 0.0 => (f.piece, true),
        _ => return None,
    };
    Some((f, grams, est))
}

/// Split a description into food phrases.
pub fn split_parts(text: &str) -> Vec<String> {
    let mut out = vec![];
    for chunk in text.split([',', ';', '+', '\n']) {
        let mut cur: Vec<&str> = vec![];
        for w in chunk.split_whitespace() {
            if matches!(w.to_lowercase().as_str(), "and" | "with" | "und" | "mit" | "&") {
                if !cur.is_empty() {
                    out.push(cur.join(" "));
                }
                cur.clear();
            } else {
                cur.push(w);
            }
        }
        if !cur.is_empty() {
            out.push(cur.join(" "));
        }
    }
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Estimate everything the food tables can place; return the rest as unknown.
pub fn local_estimate(text: &str, hhmm: &str, learned: &[Food]) -> LocalEstimate {
    let meal = meal_for_time(hhmm).to_string();
    let mut items = vec![];
    let mut unknown = vec![];
    for part in split_parts(text) {
        match place(&part, learned) {
            Some((f, grams, est)) => {
                let (kcal, prot) = nutrients(&f, grams);
                let (fiber, nut) = extras(&f, grams);
                items.push(Item {
                    name: format!("{} · {} g", capitalize(&f.name), fmt_num(grams)),
                    grams: Some(grams),
                    kcal,
                    prot,
                    est,
                    meal: meal.clone(),
                    fiber,
                    nut,
                });
            }
            None => unknown.push(part),
        }
    }
    LocalEstimate { items, unknown, meal }
}

/// kcal and protein for a typed name + grams (for the manual form). None if unknown.
pub fn quick(name: &str, grams: Option<f64>, learned: &[Food]) -> Option<(f64, f64, f64)> {
    let f = match_food(&words(name), learned)?;
    let g = match grams.filter(|g| *g > 0.0) {
        Some(g) => g,
        None if f.piece > 0.0 => f.piece,
        None => 100.0,
    };
    let (k, p) = nutrients(&f, g);
    Some((k, p, g))
}

/// Base names for autocomplete.
pub fn names() -> Vec<String> {
    TABLE.iter().map(|(n, _, _)| capitalize(n)).collect()
}

/// The food name from an item label ("Chicken breast · 200 g" → "Chicken breast").
pub fn base_name(label: &str) -> String {
    label.split(" · ").next().unwrap_or(label).trim().to_string()
}

/// What to learn from an AI item with grams, unless the base table covers it.
pub fn learn_from(item: &Item, src: &str, ts: i64) -> Option<Food> {
    let g = item.grams.filter(|g| *g > 0.0)?;
    let name = base_name(&item.name);
    if name.is_empty() || table_covers(&name) {
        return None;
    }
    Some(Food {
        name,
        kcal100: round1(item.kcal * 100.0 / g),
        prot100: round1(item.prot * 100.0 / g),
        piece: if item.est { g } else { 0.0 },
        fiber100: item.fiber.map(|x| round1(x * 100.0 / g)),
        nut100: item.nut.as_ref().map(|m| nutrients::scale(m, 100.0 / g)),
        src: src.to_string(),
        ts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str) -> Option<String> {
        match_table(&words(s)).map(|f| f.name)
    }

    #[test]
    fn matching() {
        assert_eq!(m("chicken breast").as_deref(), Some("chicken breast"));
        assert_eq!(m("chicken thigh").as_deref(), Some("chicken thigh roasted skin-off"));
        assert_eq!(m("chicken"), None, "chicken alone matches nothing");
        assert_eq!(m("eggs").as_deref(), Some("egg"));
        assert_eq!(m("prawns").as_deref(), Some("cooked prawns"));
        assert_eq!(m("shrimp").as_deref(), Some("cooked prawns"));
        assert_eq!(m("peanut butter").as_deref(), Some("Erdnussmus"));
        assert_eq!(m("PB").as_deref(), Some("Erdnussmus"));
        assert_eq!(m("toast").as_deref(), Some("wholegrain bread"));
        assert_eq!(m("bread").as_deref(), Some("wholegrain bread"));
        assert_eq!(m("chapati").as_deref(), Some("roti"));
        assert_eq!(m("beans").as_deref(), Some("kidney/white beans drained"));
        assert_eq!(m("white beans").as_deref(), Some("kidney/white beans drained"));
        assert_eq!(m("tofu").as_deref(), Some("firm tofu roasted dry"));
        assert_eq!(m("smoked tofu").as_deref(), Some("smoked tofu"));
        assert_eq!(m("flax").as_deref(), Some("ground flaxseed"));
        assert_eq!(m("olive oil").as_deref(), Some("oil"));
        assert_eq!(m("tomatoes").as_deref(), Some("tinned tomatoes"));
        assert_eq!(m("potatoes").as_deref(), Some("potato roasted dry"));
        assert_eq!(m("pepper").as_deref(), Some("bell pepper roasted dry"));
        assert_eq!(m("raspberries").as_deref(), Some("frozen raspberries"));
        assert_eq!(m("apples").as_deref(), Some("apple"));
        assert_eq!(m("Skyr").as_deref(), Some("skyr"));
        assert_eq!(m("Haferflocken"), None);
        assert_eq!(m("cheese").as_deref(), Some("cottage cheese"));
    }

    #[test]
    fn learned_first_and_strict() {
        let learned = vec![Food { name: "Protein pudding".into(), kcal100: 80.0, prot100: 10.0, piece: 200.0, ..Default::default() }];
        assert!(match_food(&words("protein pudding"), &learned).unwrap().learned);
        assert!(match_food(&words("pudding"), &learned).is_none(), "every key word must match");
        let learned = vec![Food { name: "skyr vanilla".into(), kcal100: 80.0, prot100: 9.0, ..Default::default() }];
        assert!(match_food(&words("vanilla skyr"), &learned).unwrap().learned);
        assert!(!match_food(&words("skyr"), &learned).unwrap().learned);
    }

    #[test]
    fn amounts() {
        assert_eq!(parse_amount("200g chicken").grams, Some(200.0));
        assert_eq!(parse_amount("200 g chicken").grams, Some(200.0));
        assert_eq!(parse_amount("0.2 kg rice").grams, Some(200.0));
        assert_eq!(parse_amount("0,2kg rice").grams, Some(200.0));
        assert_eq!(parse_amount("250ml milk").grams, Some(250.0));
        assert_eq!(parse_amount("1 tbsp oil").grams, Some(15.0));
        assert_eq!(parse_amount("2 EL Öl").grams, Some(30.0));
        assert_eq!(parse_amount("1 TL ghee").grams, Some(5.0));
        assert_eq!(parse_amount("2 eggs").count, Some(2.0));
        assert_eq!(parse_amount("2x roti").count, Some(2.0));
        assert_eq!(parse_amount("two eggs").count, Some(2.0));
        assert_eq!(parse_amount("half a banana").count, Some(0.5));
        assert_eq!(parse_amount("zwei Eier").count, Some(2.0));
        assert_eq!(parse_amount("200g chicken breast").food, vec!["chicken", "breast"]);
    }

    #[test]
    fn placing() {
        let (f, g, est) = place("2 eggs", &[]).unwrap();
        assert_eq!((f.name.as_str(), g, est), ("egg", 110.0, true));
        let (_, g, est) = place("banana", &[]).unwrap();
        assert_eq!((g, est), (120.0, true));
        let (_, g, est) = place("a slice of toast", &[]).unwrap();
        assert_eq!((g, est), (40.0, true));
        assert!(place("skyr", &[]).is_none(), "no amount and no piece weight");
        let (_, g, est) = place("150 g Skyr", &[]).unwrap();
        assert_eq!((g, est), (150.0, false));
    }

    #[test]
    fn estimate_examples() {
        let e = local_estimate("200g chicken breast, 2 eggs and 1 banana", "12:00", &[]);
        assert_eq!(e.items.len(), 3);
        assert!(e.unknown.is_empty());
        assert_eq!(e.items[0].name, "Chicken breast · 200 g");
        assert_eq!(e.items[0].kcal, 330.0);
        assert_eq!(e.items[0].prot, 62.0);
        assert!(!e.items[0].est);
        assert_eq!(e.items[1].grams, Some(110.0));
        assert!(e.items[1].est);
        assert_eq!(e.items[2].grams, Some(120.0));
        assert_eq!(e.meal, "Lunch");

        let e = local_estimate("some chicken", "08:00", &[]);
        assert!(e.items.is_empty());
        assert_eq!(e.unknown, vec!["some chicken"]);
        assert_eq!(e.meal, "Breakfast");

        let e = local_estimate("1 tbsp olive oil + 30g peanut butter", "20:00", &[]);
        assert_eq!(e.items.len(), 2);
        assert_eq!(e.items[0].kcal, 133.0);
        assert_eq!(e.items[1].kcal, 180.0);

        let e = local_estimate("150 g Skyr mit 40g Haferflocken", "09:00", &[]);
        assert_eq!(e.items.len(), 1);
        assert_eq!(e.unknown, vec!["40g Haferflocken"]);
    }

    #[test]
    fn splitting() {
        assert_eq!(split_parts("a; b\nc + d with e und f"), vec!["a", "b", "c", "d", "e", "f"]);
        assert_eq!(split_parts("  ,, "), Vec::<String>::new());
    }

    #[test]
    fn learning() {
        let item = Item {
            name: "Protein pudding · 200 g".into(),
            grams: Some(200.0),
            kcal: 160.0,
            prot: 20.0,
            est: true,
            meal: "Snack".into(),
            fiber: None,
            nut: None,
        };
        let f = learn_from(&item, "claude", 1).unwrap();
        assert_eq!((f.name.as_str(), f.kcal100, f.prot100, f.piece), ("Protein pudding", 80.0, 10.0, 200.0));
        let item = Item {
            name: "Pudding".into(),
            grams: Some(100.0),
            kcal: 80.0,
            prot: 3.0,
            est: false,
            meal: "Snack".into(),
            fiber: None,
            nut: None,
        };
        assert_eq!(learn_from(&item, "gemini", 1).unwrap().piece, 0.0);
        let covered = Item {
            name: "Chicken breast".into(),
            grams: Some(100.0),
            kcal: 165.0,
            prot: 31.0,
            est: false,
            meal: "Lunch".into(),
            fiber: None,
            nut: None,
        };
        assert!(learn_from(&covered, "claude", 1).is_none());
        let no_grams = Item { grams: None, ..item };
        assert!(learn_from(&no_grams, "claude", 1).is_none());
    }

    #[test]
    fn fiber_and_micros() {
        assert_eq!(EXTRA.len(), TABLE.len(), "every table food has fiber and micros");
        for (n, _, _) in TABLE {
            assert!(EXTRA.iter().any(|(e, _, _)| e == n), "{n} missing from EXTRA");
        }
        let e = local_estimate("50g oats, 1 banana", "08:00", &[]);
        assert_eq!(e.items[0].fiber, Some(5.1));
        assert_eq!(e.items[1].fiber, Some(3.1));
        let m = e.items[1].nut.clone().unwrap();
        assert_eq!(m["vitC"], 10.4);
        assert_eq!(richest("fiber", 2), vec!["Ground flaxseed", "Soy granulate dry"]);
        assert_eq!(richest("vitC", 1), vec!["Bell pepper roasted dry"]);
        assert!(table_text().contains("oats 370/13/10.1"));
    }

    #[test]
    fn quick_fill() {
        assert_eq!(quick("chicken breast", Some(100.0), &[]), Some((165.0, 31.0, 100.0)));
        assert_eq!(quick("egg", None, &[]), Some((79.0, 6.9, 55.0)));
        assert_eq!(quick("mystery", Some(100.0), &[]), None);
    }
}
