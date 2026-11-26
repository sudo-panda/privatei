//! The app facade: every screen's data and every action, computed here.
//! The UI only renders what these return.

use crate::ai::{self, AiAnswer, AiConfig, AiRequest, Image, RuleChange};
use crate::dates;
use crate::food::{self, round1, Item, LocalEstimate};
use crate::meds;
use crate::model::*;
use crate::nutrients::{self, Nut};
use crate::sleep::{self, SleepView};
use crate::store::{new_id, Store};
use crate::training;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, String>;

pub const BUDGET_CHF: f64 = 80.0;

#[derive(Debug, Clone)]
pub enum Clock {
    System,
    Fixed { today: String, hhmm: String },
}

pub struct Core {
    pub store: Store,
    /// National food databases (None until opened).
    pub foods: Option<crate::fooddb::FoodDb>,
    pub cache_dir: PathBuf,
    clock: Clock,
    last_ts: std::cell::Cell<i64>,
}

// ---------- views ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRow {
    pub id: String,
    pub name: String,
    pub grams: Option<f64>,
    pub kcal: f64,
    pub prot: f64,
    pub fiber: Option<f64>,
    pub meal: String,
    pub est: bool,
    pub from_plan: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MealGroup {
    pub meal: String,
    pub kcal: f64,
    pub prot: f64,
    /// Sum of the rows that know their fiber; None when none of them do (unknown, not 0).
    pub fiber: Option<f64>,
    pub rows: Vec<LogRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanMealView {
    pub idx: usize,
    pub slot: String,
    pub name: String,
    pub kcal: f64,
    pub prot: f64,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoseView {
    pub med_id: String,
    pub group_id: String,
    pub name: String,
    pub dose: String,
    pub time: String,
    pub status: String,
    pub taken: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutRow {
    pub id: String,
    pub kind: String,
    pub mins: f64,
    pub kcal: f64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutEntry {
    pub id: String,
    pub date: String,
    pub kind: String,
    pub mins: f64,
    pub kcal: f64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayView {
    pub date: String,
    pub today: String,
    pub is_today: bool,
    pub kcal: f64,
    pub prot: f64,
    pub target: f64,
    pub kcal_target: f64,
    pub prot_floor: f64,
    pub burn: f64,
    pub add_burn: bool,
    pub over: bool,
    pub prot_met: bool,
    /// Sum of known fiber; `fiber_known` says whether any row had fiber data.
    pub fiber: f64,
    pub fiber_floor: f64,
    pub fiber_known: bool,
    pub fiber_met: bool,
    /// Every tracked nutrient for the day (the Log shows few; History shows all).
    pub nutrients: Vec<NutView>,
    pub plan: Vec<PlanMealView>,
    pub meals: Vec<MealGroup>,
    pub meds: Vec<DoseView>,
    pub workouts: Vec<WorkoutRow>,
    /// Set when this day is a planned shopping day.
    pub shop: Option<ShopDay>,
    /// Last night: the sleep that ended on this date.
    pub sleep: Option<SleepView>,
    pub can_copy_yesterday: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogInput {
    pub meal: String,
    pub name: String,
    #[serde(default)]
    pub grams: Option<f64>,
    pub kcal: f64,
    pub prot: f64,
    #[serde(default)]
    pub est: bool,
    #[serde(default)]
    pub fiber: Option<f64>,
    #[serde(default)]
    pub nut: Option<Nut>,
    #[serde(default)]
    pub food_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFood {
    pub name: String,
    pub grams: Option<f64>,
    pub kcal: f64,
    pub prot: f64,
    pub est: bool,
    pub fiber: Option<f64>,
    pub nut: Option<Nut>,
    pub food_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateView {
    pub items: Vec<DraftItem>,
    pub unknown: Vec<String>,
    pub note: String,
    pub provider: Option<String>,
    pub provider_text: Option<String>,
    pub error: Option<String>,
    pub meal: String,
}

/// One row of the Add food draft. Matched rows carry their database food, other foods
/// it could be, and that food's portions, so the row can be changed like in a food diary.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DraftItem {
    #[serde(flatten)]
    pub item: Item,
    /// Where the numbers come from: "USDA", "Built-in table"…; None for an AI estimate.
    pub source_name: Option<String>,
    pub alts: Vec<FoodChoice>,
    pub portions: Vec<crate::fooddb::Portion>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FoodChoice {
    #[serde(rename = "ref")]
    pub reference: String,
    pub name: String,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickFill {
    pub kcal: f64,
    pub prot: f64,
    pub fiber: Option<f64>,
    pub grams: f64,
    pub matched: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MedView {
    pub id: String,
    pub group_id: String,
    pub name: String,
    pub dose: String,
    pub freq: String,
    pub days: Vec<u8>,
    pub dom: Option<u8>,
    pub time: String,
    pub from: String,
    pub until: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MedsView {
    pub daily: Vec<MedView>,
    pub days: Vec<MedView>,
    pub monthly: Vec<MedView>,
    pub stopped: Vec<MedView>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MedInput {
    #[serde(default)]
    pub group_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub dose: String,
    pub freq: String,
    #[serde(default)]
    pub days: Vec<u8>,
    #[serde(default)]
    pub dom: Option<u8>,
    #[serde(default)]
    pub time: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMedResult {
    pub group_id: String,
    pub started_new: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockRow {
    pub id: String,
    pub name: String,
    pub qty: f64,
    pub unit: String,
    pub cat: String,
    pub added: String,
    pub use_by: Option<String>,
    pub age_days: i64,
    pub days_left: Option<i64>,
    pub soon: bool,
    pub eat_today: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockSection {
    pub cat: String,
    pub items: Vec<StockRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockView {
    pub sections: Vec<StockSection>,
    pub soon_count: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRow {
    pub id: String,
    pub name: String,
    pub qty: String,
    pub cat: String,
    pub price: Option<f64>,
    pub trip: u8,
    pub checked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGroup {
    pub cat: String,
    pub items: Vec<ListRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopView {
    pub trip: u8,
    pub budget: f64,
    pub spend: f64,
    pub trip_spend: f64,
    pub over_budget: bool,
    pub groups: Vec<ListGroup>,
    pub in_stock: Vec<ListRow>,
    pub checked_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HaveMatch {
    pub list_id: String,
    pub list_name: String,
    pub stock_id: String,
    pub stock_name: String,
    pub stock_qty: f64,
    pub stock_unit: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDay {
    pub date: String,
    pub dow: u8,
    pub is_today: bool,
    pub meals: Vec<PlanMeal>,
    pub kcal: f64,
    pub prot: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekView {
    pub days: Vec<PlanDay>,
    pub avg_kcal: f64,
    pub avg_prot: f64,
    pub planned_days: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistDay {
    pub date: String,
    pub kcal: f64,
    pub prot: f64,
    pub fiber: Option<f64>,
    pub nut: Option<Nut>,
    pub mins: f64,
    pub taken: usize,
    pub due: usize,
    pub used: bool,
    pub prot_met: bool,
    pub fiber_met: bool,
    pub over: bool,
    pub target: f64,
    /// The night that ended on this day, if logged.
    pub sleep: Option<SleepView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NutView {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub group: String,
    /// "target" (reach it), "limit" (stay under) or "info".
    pub kind: String,
    pub amount: f64,
    pub reference: f64,
    /// Percent of the reference; None when unknown or there's no reference.
    pub pct: Option<f64>,
    /// Share (0–1) of entries/days that had a value for this nutrient.
    pub coverage: f64,
}

/// Build views for every catalog nutrient from summed amounts and how many
/// entries knew each one, out of `total` entries.
fn nut_views(sum: &Nut, known: &BTreeMap<String, usize>, total: usize, fiber_ref: f64) -> Vec<NutView> {
    nutrients::CATALOG
        .iter()
        .map(|d| {
            let k = known.get(d.key).copied().unwrap_or(0);
            let amount = sum.get(d.key).copied().unwrap_or(0.0);
            let reference = if d.key == "fiber" { fiber_ref } else { d.reference };
            NutView {
                key: d.key.into(),
                label: d.label.into(),
                unit: d.unit.into(),
                group: d.group.into(),
                kind: match d.kind {
                    nutrients::Kind::Target => "target",
                    nutrients::Kind::Limit => "limit",
                    nutrients::Kind::Info => "info",
                }
                .into(),
                amount: nutrients::round(amount),
                reference,
                pct: (k > 0 && reference > 0.0).then(|| (amount / reference * 100.0).round()),
                coverage: if total == 0 { 0.0 } else { (k as f64 / total as f64 * 100.0).round() / 100.0 },
            }
        })
        .collect()
}

/// All nutrients of a recipe (sum of ingredients), including fiber.
fn recipe_nut(r: &Recipe) -> Nut {
    let mut n = Nut::new();
    for i in &r.ingredients {
        if let Some(m) = &i.nut {
            nutrients::add(&mut n, m);
        }
        if let Some(f) = i.fiber {
            *n.entry("fiber".into()).or_default() += f;
        }
    }
    n
}

fn recipe_view(id: &str, r: &Recipe) -> RecipeView {
    let raw: f64 = r.ingredients.iter().map(|i| i.grams).sum();
    let grams = r.cooked_grams.unwrap_or(raw);
    let kcal: f64 = r.ingredients.iter().map(|i| i.kcal).sum();
    let prot: f64 = r.ingredients.iter().map(|i| i.prot).sum();
    let fiber = r.ingredients.iter().any(|i| i.fiber.is_some()).then(|| r.ingredients.iter().filter_map(|i| i.fiber).sum::<f64>());
    let s = r.servings.max(1e-9);
    // A nutrient counts as known only if every ingredient has it (otherwise the sum understates it).
    let known = nutrients::CATALOG
        .iter()
        .filter(|d| {
            r.ingredients.iter().all(|i| {
                if d.key == "fiber" {
                    i.fiber.is_some()
                } else {
                    i.nut.as_ref().is_some_and(|n| n.contains_key(d.key))
                }
            })
        })
        .count();
    RecipeView {
        id: id.into(),
        name: r.name.clone(),
        servings: r.servings,
        cooked_grams: r.cooked_grams,
        note: r.note.clone(),
        ingredients: r.ingredients.clone(),
        total: Amounts { grams: grams.round(), kcal: kcal.round(), prot: round1(prot), fiber: fiber.map(round1) },
        per_serving: Amounts {
            grams: (grams / s).round(),
            kcal: (kcal / s).round(),
            prot: round1(prot / s),
            fiber: fiber.map(|f| round1(f / s)),
        },
        known,
    }
}

/// A log row's nutrients including fiber.
fn row_nut(l: &Log) -> Nut {
    let mut n = l.nut.clone().unwrap_or_default();
    if let Some(f) = l.fiber {
        n.insert("fiber".into(), f);
    }
    n
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistSummary {
    pub avg_kcal: f64,
    pub avg_prot: f64,
    pub avg_fiber: Option<f64>,
    pub prot_days: usize,
    pub fiber_days: usize,
    pub food_days: usize,
    /// Percent of due doses taken, over days the app was used. None if nothing was due.
    pub adherence: Option<f64>,
    pub days_trained: usize,
    pub days_used: usize,
    pub nutrients: Vec<NutView>,
    /// Over the nights logged in the period; None with no nights.
    pub avg_sleep_mins: Option<f64>,
    pub avg_bed: Option<String>,
    pub avg_wake: Option<String>,
    pub sleep_nights: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MedHist {
    pub name: String,
    pub taken: usize,
    pub due: usize,
    pub missed: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    /// "warn", "info" or "good"
    pub level: String,
    pub title: String,
    pub detail: String,
}

impl Insight {
    fn warn(t: &str, d: &str) -> Insight {
        Insight { level: "warn".into(), title: t.into(), detail: d.into() }
    }
    fn info(t: &str, d: &str) -> Insight {
        Insight { level: "info".into(), title: t.into(), detail: d.into() }
    }
    fn good(t: &str, d: &str) -> Insight {
        Insight { level: "good".into(), title: t.into(), detail: d.into() }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub range: usize,
    pub kcal_target: f64,
    pub prot_floor: f64,
    pub fiber_floor: f64,
    /// Newest first.
    pub days: Vec<HistDay>,
    pub summary: HistSummary,
    pub meds: Vec<MedHist>,
    pub insights: Vec<Insight>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanAheadMeal {
    pub idx: usize,
    pub slot: String,
    pub name: String,
    pub kcal: f64,
    pub prot: f64,
    pub fiber: Option<f64>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanAheadDay {
    pub date: String,
    pub dow: u8,
    pub is_today: bool,
    /// Days from today (0 = today).
    pub offset: i64,
    pub meals: Vec<PlanAheadMeal>,
    pub kcal: f64,
    pub prot: f64,
    pub fiber: f64,
    pub over: bool,
    pub prot_met: bool,
    /// Slots (Breakfast/Lunch/Snack/Dinner) with nothing planned.
    pub empty_slots: Vec<String>,
    /// Set when this is a planned shopping day.
    pub shop: Option<ShopDay>,
}

/// A planned shopping day: which trip, and what's still to buy on it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShopDay {
    pub trip: u8,
    /// Items on that trip's list not yet ticked.
    pub items: usize,
    /// Their known prices added up (CHF).
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanAhead {
    pub days: Vec<PlanAheadDay>,
    pub kcal_target: f64,
    pub prot_floor: f64,
    pub fiber_floor: f64,
    pub planned_days: usize,
    pub avg_kcal: f64,
    pub avg_prot: f64,
    /// The newest planned day before today, if any (for "copy last week").
    pub has_earlier: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amounts {
    pub grams: f64,
    pub kcal: f64,
    pub prot: f64,
    pub fiber: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeView {
    pub id: String,
    pub name: String,
    pub servings: f64,
    pub cooked_grams: Option<f64>,
    pub note: String,
    pub ingredients: Vec<Ingredient>,
    pub total: Amounts,
    pub per_serving: Amounts,
    /// How many catalog nutrients every ingredient has a value for.
    pub known: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeInput {
    pub name: String,
    pub servings: f64,
    #[serde(default)]
    pub cooked_grams: Option<f64>,
    #[serde(default)]
    pub ingredients: Vec<Ingredient>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingSummary {
    pub week_sessions: usize,
    pub week_mins: f64,
    pub week_kcal: f64,
    /// Days since the last session; None if there is none.
    pub last_days_ago: Option<i64>,
    /// Minutes per sport over the last 28 days, biggest first.
    pub by_kind: Vec<(String, f64)>,
    /// Minutes per day for the last 28 days, oldest first.
    pub daily: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsView {
    pub order: Vec<String>,
    pub gemini_key_set: bool,
    pub gemini_model: String,
    pub claude_model: String,
    pub claude_path: String,
    pub claude_found: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsInput {
    pub order: Vec<String>,
    /// Empty keeps the stored key.
    #[serde(default)]
    pub gemini_key: String,
    #[serde(default)]
    pub clear_gemini_key: bool,
    #[serde(default)]
    pub gemini_model: String,
    #[serde(default)]
    pub claude_model: String,
    #[serde(default)]
    pub claude_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnedFood {
    pub id: String,
    pub name: String,
    pub kcal100: f64,
    pub prot100: f64,
    pub piece: f64,
    pub src: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesReply {
    pub reply: String,
    pub change: Option<RuleChange>,
    pub provider_text: String,
}

/// Work prepared under the lock; the AI call itself runs without it.
#[derive(Debug, Clone)]
pub struct Pending {
    pub local: LocalEstimate,
    pub ai: Option<(AiConfig, AiRequest)>,
    pub hhmm: String,
}

pub fn worked_it_out(p: ai::Provider) -> String {
    format!("{} worked it out", p.label())
}

fn valid_meal(m: &str, hhmm: &str) -> String {
    if MEALS.contains(&m) {
        m.to_string()
    } else {
        meal_for_time(hhmm).to_string()
    }
}

fn check_date(d: &str) -> Result<()> {
    if dates::is_date(d) {
        Ok(())
    } else {
        Err("That date isn't valid.".into())
    }
}

/// "4 pots" → (4, "pots"); "1 kg" → (1, "kg"); "500g" → (500, "g"); "" → (1, "").
pub fn parse_qty(s: &str) -> (f64, String) {
    let s = s.trim();
    let end = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(s.len());
    let n = s[..end].replace(',', ".").parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0);
    match n {
        Some(n) => (n, s[end..].trim().to_string()),
        None => (1.0, String::new()),
    }
}

impl Core {
    pub fn open(data_dir: &Path, cache_dir: &Path, device: &str) -> Result<Core> {
        let store = Store::open(&data_dir.join("privatei.sqlite"), device)?;
        Ok(Core { store, foods: None, cache_dir: cache_dir.to_path_buf(), clock: Clock::System, last_ts: Default::default() })
    }

    pub fn in_memory() -> Core {
        Core {
            store: Store::open_in_memory("test").expect("in-memory db"),
            foods: None,
            cache_dir: std::env::temp_dir().join("privatei-cache"),
            clock: Clock::System,
            last_ts: Default::default(),
        }
    }

    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    pub fn today(&self) -> String {
        match &self.clock {
            Clock::System => dates::today(),
            Clock::Fixed { today, .. } => today.clone(),
        }
    }

    pub fn hhmm(&self) -> String {
        match &self.clock {
            Clock::System => dates::now_hhmm(),
            Clock::Fixed { hhmm, .. } => hhmm.clone(),
        }
    }

    /// Strictly increasing ms timestamps, so rows logged in the same ms keep their order.
    fn now(&self) -> i64 {
        let t = dates::now_ms().max(self.last_ts.get() + 1);
        self.last_ts.set(t);
        t
    }

    /// Reserve `n` consecutive timestamps; returns the first.
    fn reserve_ts(&self, n: usize) -> i64 {
        let t = self.now();
        self.last_ts.set(t + n as i64);
        t
    }

    fn learned(&self) -> Vec<Food> {
        self.store.all::<Food>().map(|(_, f)| f.clone()).collect()
    }

    fn ai_config(&self) -> AiConfig {
        AiConfig::from_settings(&self.store.ai_settings(), self.cache_dir.join("ai"))
    }

    // ---------- Today ----------

    fn plan_log_ids(&self, date: &str) -> HashSet<String> {
        self.store.get::<Plan>(date).map(|p| p.meals.iter().filter_map(|m| m.log_id.clone()).collect()).unwrap_or_default()
    }

    pub fn totals(&self, date: &str) -> (f64, f64) {
        let (k, p) = self.store.all::<Log>().filter(|(_, l)| l.date == date).fold((0.0, 0.0), |(k, p), (_, l)| (k + l.kcal, p + l.prot));
        (k.round(), round1(p))
    }

    pub fn burn_on(&self, date: &str) -> (f64, f64) {
        self.store.all::<Workout>().filter(|(_, w)| w.date == date).fold((0.0, 0.0), |(k, m), (_, w)| (k + w.kcal, m + w.mins))
    }

    fn doses(&self, date: &str) -> Vec<DoseView> {
        let today = self.today();
        let hhmm = self.hhmm();
        let mut v: Vec<DoseView> = self
            .store
            .all::<Med>()
            .filter(|(_, m)| meds::is_due(m, date))
            .map(|(id, m)| {
                let taken = self.store.get::<MedLog>(&format!("{date}_{id}")).is_some();
                DoseView {
                    med_id: id.clone(),
                    group_id: m.group_id.clone(),
                    name: m.name.clone(),
                    dose: m.dose.clone(),
                    time: m.time.clone(),
                    status: meds::status(m, date, &today, &hhmm, taken).into(),
                    taken,
                }
            })
            .collect();
        v.sort_by(|a, b| (a.time.is_empty(), &a.time, &a.name).cmp(&(b.time.is_empty(), &b.time, &b.name)));
        v
    }

    pub fn day(&self, date: &str) -> Result<DayView> {
        check_date(date)?;
        let today = self.today();
        let s = self.store.settings();
        let plan_ids = self.plan_log_ids(date);
        // Every meal is always there, empty or not, so the Log keeps the same shape all day.
        let mut groups: BTreeMap<usize, MealGroup> = MEALS
            .iter()
            .enumerate()
            .map(|(i, m)| (i, MealGroup { meal: (*m).into(), kcal: 0.0, prot: 0.0, fiber: None, rows: vec![] }))
            .collect();
        let mut logs: Vec<(&String, &Log)> = self.store.all::<Log>().filter(|(_, l)| l.date == date).collect();
        logs.sort_by_key(|(_, l)| l.ts);
        for (id, l) in logs {
            let idx = MEALS.iter().position(|m| *m == l.meal).unwrap_or(1);
            let g = groups.get_mut(&idx).expect("every meal has a group");
            g.kcal += l.kcal;
            g.prot += l.prot;
            if let Some(f) = l.fiber {
                g.fiber = Some(g.fiber.unwrap_or(0.0) + f);
            }
            g.rows.push(LogRow {
                id: id.clone(),
                name: l.name.clone(),
                grams: l.grams,
                kcal: l.kcal,
                prot: l.prot,
                fiber: l.fiber,
                meal: l.meal.clone(),
                est: l.est,
                from_plan: plan_ids.contains(id),
            });
        }
        let meals: Vec<MealGroup> = groups
            .into_values()
            .map(|mut g| {
                g.kcal = g.kcal.round();
                g.prot = round1(g.prot);
                g.fiber = g.fiber.map(round1);
                g
            })
            .collect();
        let (kcal, prot) = self.totals(date);
        let (burn, _) = self.burn_on(date);
        let (mut fiber, mut fiber_known) = (0.0, false);
        let mut sum = Nut::new();
        let mut known: BTreeMap<String, usize> = BTreeMap::new();
        let mut rows = 0;
        for (_, l) in self.store.all::<Log>().filter(|(_, l)| l.date == date) {
            rows += 1;
            if let Some(f) = l.fiber {
                fiber += f;
                fiber_known = true;
            }
            let n = row_nut(l);
            for k in n.keys() {
                *known.entry(k.clone()).or_default() += 1;
            }
            nutrients::add(&mut sum, &n);
        }
        let nutrient_views = nut_views(&sum, &known, rows, s.fiber_floor);
        let target = training::day_target(s.kcal_target, burn, s.add_burn && burn > 0.0);
        let plan = self
            .store
            .get::<Plan>(date)
            .map(|p| {
                p.meals
                    .iter()
                    .enumerate()
                    .map(|(idx, m)| PlanMealView {
                        idx,
                        slot: m.slot.clone(),
                        name: m.name.clone(),
                        kcal: m.kcal,
                        prot: m.prot,
                        done: m.done,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut workouts: Vec<(&String, &Workout)> = self.store.all::<Workout>().filter(|(_, w)| w.date == date).collect();
        workouts.sort_by_key(|(_, w)| w.ts);
        let own_rows = meals.iter().flat_map(|g| &g.rows).filter(|r| !r.from_plan).count();
        let yesterday = dates::add_days(date, -1);
        let y_ids = self.plan_log_ids(&yesterday);
        let y_rows = self.store.all::<Log>().filter(|(id, l)| l.date == yesterday && !y_ids.contains(*id)).count();
        Ok(DayView {
            date: date.into(),
            is_today: date == today,
            today,
            kcal,
            prot,
            target,
            kcal_target: s.kcal_target,
            prot_floor: s.prot_floor,
            burn: burn.round(),
            add_burn: s.add_burn,
            over: kcal > target,
            prot_met: prot >= s.prot_floor,
            fiber: round1(fiber),
            fiber_floor: s.fiber_floor,
            fiber_known,
            fiber_met: fiber_known && fiber >= s.fiber_floor,
            nutrients: nutrient_views,
            plan,
            meals,
            meds: self.doses(date),
            workouts: workouts
                .into_iter()
                .map(|(id, w)| WorkoutRow { id: id.clone(), kind: w.kind.clone(), mins: w.mins, kcal: w.kcal, name: w.name.clone() })
                .collect(),
            shop: self.shop_day(date),
            sleep: self.store.get::<Sleep>(date).and_then(sleep::view),
            can_copy_yesterday: own_rows == 0 && y_rows > 0,
        })
    }

    /// Save the night that ended on `date`: bed and wake as "HH:MM".
    pub fn set_sleep(&mut self, date: &str, bed: &str, wake: &str) -> Result<()> {
        check_date(date)?;
        if date > self.today().as_str() {
            return Err("You can't log sleep for a day that hasn't come yet.".into());
        }
        let (Some(b), Some(w)) = (sleep::parse_hhmm(bed), sleep::parse_hhmm(wake)) else {
            return Err("Use times like 23:30 and 07:00.".into());
        };
        let (bed, wake) = (sleep::fmt_hhmm(b), sleep::fmt_hhmm(w));
        if sleep::duration(&bed, &wake).is_none() {
            return Err("Bedtime and wake-up time can't be the same, or more than 20 hours apart.".into());
        }
        let ts = self.now();
        self.store.put(date, Sleep { bed, wake, ts })
    }

    pub fn clear_sleep(&mut self, date: &str) -> Result<()> {
        check_date(date)?;
        if self.store.get::<Sleep>(date).is_some() {
            self.store.delete::<Sleep>(date)?;
        }
        Ok(())
    }

    pub fn add_log(&mut self, date: &str, input: LogInput) -> Result<String> {
        check_date(date)?;
        let name = ai::clip(&input.name, 80);
        if name.is_empty() {
            return Err("Give the food a name.".into());
        }
        if !(input.kcal > 0.0 && input.kcal < 5000.0) {
            return Err("Calories must be above 0 and below 5000.".into());
        }
        if !(0.0..=500.0).contains(&input.prot) {
            return Err("Protein must be between 0 and 500 g.".into());
        }
        let id = new_id();
        let log = Log {
            date: date.into(),
            meal: valid_meal(&input.meal, &self.hhmm()),
            name,
            grams: input.grams.filter(|g| *g > 0.0 && g.is_finite()),
            kcal: input.kcal.round(),
            prot: round1(input.prot),
            est: input.est,
            ts: self.now(),
            fiber: input.fiber.filter(|f| f.is_finite() && *f >= 0.0).map(|f| round1(f.min(300.0))),
            nut: input.nut.as_ref().map(nutrients::clean).filter(|n| !n.is_empty()),
            food_ref: input.food_ref.clone(),
        };
        self.store.put(&id, log)?;
        Ok(id)
    }

    /// Edit a logged food. Keeps its date and time; a ticked plan meal follows the edit.
    pub fn update_log(&mut self, id: &str, input: LogInput) -> Result<()> {
        let mut l = self.store.get::<Log>(id).cloned().ok_or("That entry no longer exists.")?;
        let name = ai::clip(&input.name, 80);
        if name.is_empty() {
            return Err("Give the food a name.".into());
        }
        // 0 is fine (water, black coffee): add_food logs those too.
        if !(input.kcal >= 0.0 && input.kcal < 5000.0) {
            return Err("Calories must be between 0 and 5000.".into());
        }
        if !(0.0..=500.0).contains(&input.prot) {
            return Err("Protein must be between 0 and 500 g.".into());
        }
        let grams = input.grams.filter(|g| *g > 0.0 && g.is_finite());
        // "Skyr · 200 g" changed to 100 g reads "Skyr · 100 g", not the old amount.
        let name = match (l.grams, grams) {
            (Some(old), Some(new)) if old != new => {
                let tail = format!(" · {} g", food::fmt_num(old.round()));
                match name.strip_suffix(&tail) {
                    Some(base) => format!("{base} · {} g", food::fmt_num(new.round())),
                    None => name,
                }
            }
            _ => name,
        };
        // Changing the amount rescales everything we know about the portion.
        if let (Some(old), Some(new), Some(m)) = (l.grams, grams, l.nut.as_ref()) {
            if old > 0.0 && input.nut.is_none() {
                l.nut = Some(nutrients::scale(m, new / old));
            }
        }
        if let Some(n) = &input.nut {
            l.nut = Some(nutrients::clean(n)).filter(|n| !n.is_empty());
        }
        l.name = name;
        l.meal = valid_meal(&input.meal, &self.hhmm());
        l.grams = grams;
        l.kcal = input.kcal.round();
        l.prot = round1(input.prot);
        l.fiber = input.fiber.filter(|f| f.is_finite() && *f >= 0.0).map(|f| round1(f.min(300.0)));
        l.est = input.est;
        let date = l.date.clone();
        let (name, kcal, prot, fiber, slot) = (l.name.clone(), l.kcal, l.prot, l.fiber, l.meal.clone());
        self.store.put(id, l)?;
        if let Some(mut plan) = self.store.get::<Plan>(&date).cloned() {
            let mut changed = false;
            for m in plan.meals.iter_mut().filter(|m| m.log_id.as_deref() == Some(id)) {
                (m.name, m.kcal, m.prot, m.fiber, m.slot) = (name.clone(), kcal, prot, fiber, slot.clone());
                changed = true;
            }
            if changed {
                self.store.put(&date, plan)?;
            }
        }
        Ok(())
    }

    /// Edit one planned meal; if it was already ticked, its log row follows.
    pub fn update_plan_meal(&mut self, date: &str, idx: usize, expect: Option<&str>, meal: PlanMeal) -> Result<()> {
        let mut meals = self.store.get::<Plan>(date).map(|p| p.meals.clone()).ok_or("There's no plan for that day.")?;
        let old = plan_meal_at(&meals, idx, expect)?.clone();
        meals[idx] = PlanMeal { done: old.done, log_id: old.log_id.clone(), ..meal };
        self.set_plan(date, meals)?;
        if let Some(lid) = old.log_id {
            if let Some(mut l) = self.store.get::<Log>(&lid).cloned() {
                let m = &self.store.get::<Plan>(date).unwrap().meals[idx];
                (l.name, l.kcal, l.prot, l.fiber, l.meal) = (m.name.clone(), m.kcal, m.prot, m.fiber, m.slot.clone());
                self.store.put(&lid, l)?;
            }
        }
        Ok(())
    }

    /// Save a draft's items (already cleaned). Returns how many were saved.
    pub fn save_items(&mut self, date: &str, items: Vec<Item>) -> Result<usize> {
        check_date(date)?;
        let hhmm = self.hhmm();
        let items: Vec<Item> = items.into_iter().take(30).collect();
        let now = self.reserve_ts(items.len());
        self.store.batch(|s| {
            let mut n = 0;
            for (i, it) in items.iter().enumerate() {
                let name = ai::clip(&it.name, 80);
                if name.is_empty() {
                    continue;
                }
                let log = Log {
                    date: date.into(),
                    meal: valid_meal(&it.meal, &hhmm),
                    name,
                    grams: it.grams.filter(|g| *g > 0.0 && g.is_finite()),
                    kcal: it.kcal.clamp(0.0, 5000.0).round(),
                    prot: round1(it.prot.clamp(0.0, 500.0)),
                    est: it.est,
                    ts: now + i as i64,
                    fiber: it.fiber.filter(|f| f.is_finite()).map(|f| round1(f.clamp(0.0, 300.0))),
                    nut: it.nut.as_ref().map(nutrients::clean).filter(|n| !n.is_empty()),
                    food_ref: it.food_ref.as_deref().map(|r| ai::clip(r, 80)).filter(|r| !r.is_empty()),
                };
                s.put(&new_id(), log)?;
                n += 1;
            }
            Ok(n)
        })
    }

    pub fn delete_log(&mut self, id: &str) -> Result<()> {
        let Some(date) = self.store.get::<Log>(id).map(|l| l.date.clone()) else {
            return Ok(());
        };
        if let Some(mut plan) = self.store.get::<Plan>(&date).cloned() {
            let mut changed = false;
            for m in plan.meals.iter_mut().filter(|m| m.log_id.as_deref() == Some(id)) {
                m.done = false;
                m.log_id = None;
                changed = true;
            }
            if changed {
                self.store.put(&date, plan)?;
            }
        }
        self.store.delete::<Log>(id)?;
        Ok(())
    }

    /// Tick or untick a planned meal. Ticking creates a log row; unticking deletes it.
    pub fn toggle_plan(&mut self, date: &str, idx: usize, expect: Option<&str>) -> Result<bool> {
        check_date(date)?;
        let mut plan = self.store.get::<Plan>(date).cloned().ok_or("There's no plan for that day.")?;
        let hhmm = self.hhmm();
        let now = self.now();
        plan_meal_at(&plan.meals, idx, expect)?;
        let m = &mut plan.meals[idx];
        let done = if m.done {
            if let Some(lid) = m.log_id.take() {
                self.store.delete::<Log>(&lid)?;
            }
            m.done = false;
            false
        } else {
            let id = new_id();
            let log = Log {
                date: date.into(),
                meal: valid_meal(&m.slot, &hhmm),
                name: m.name.clone(),
                grams: None,
                kcal: m.kcal,
                prot: m.prot,
                est: false,
                ts: now,
                fiber: m.fiber,
                nut: None,
                food_ref: None,
            };
            self.store.put(&id, log)?;
            m.done = true;
            m.log_id = Some(id);
            true
        };
        self.store.put(date, plan)?;
        Ok(done)
    }

    /// Replace a day's planned meals, keeping ticks of meals that stay.
    pub fn set_plan(&mut self, date: &str, meals: Vec<PlanMeal>) -> Result<()> {
        check_date(date)?;
        let old = self.store.get::<Plan>(date).cloned().unwrap_or_default();
        let keep: HashSet<String> = meals.iter().filter_map(|m| m.log_id.clone()).collect();
        for m in &old.meals {
            if let Some(lid) = &m.log_id {
                if !keep.contains(lid) {
                    self.store.delete::<Log>(lid)?;
                }
            }
        }
        let meals: Vec<PlanMeal> = meals
            .into_iter()
            .take(12)
            .filter_map(|mut m| {
                m.name = ai::clip(&m.name, 80);
                if m.name.is_empty() {
                    return None;
                }
                m.slot = valid_meal(&m.slot, "12:00");
                m.kcal = m.kcal.clamp(0.0, 5000.0).round();
                m.prot = round1(m.prot.clamp(0.0, 500.0));
                if m.log_id.as_deref().is_none_or(|l| self.store.get::<Log>(l).is_none()) {
                    m.log_id = None;
                    m.done = false;
                }
                Some(m)
            })
            .collect();
        self.store.put(date, Plan { meals, shop: old.shop })
    }

    /// Copy the previous day's own (non-plan) log rows when this day has none.
    pub fn copy_yesterday(&mut self, date: &str) -> Result<usize> {
        check_date(date)?;
        let ids = self.plan_log_ids(date);
        if self.store.all::<Log>().any(|(id, l)| l.date == date && !ids.contains(id)) {
            return Err("This day already has food logged.".into());
        }
        let prev = dates::add_days(date, -1);
        let prev_ids = self.plan_log_ids(&prev);
        let mut rows: Vec<Log> =
            self.store.all::<Log>().filter(|(id, l)| l.date == prev && !prev_ids.contains(*id)).map(|(_, l)| l.clone()).collect();
        rows.sort_by_key(|l| l.ts);
        let n = rows.len();
        let now = self.reserve_ts(n);
        self.store.batch(|s| {
            for (i, mut l) in rows.into_iter().enumerate() {
                l.date = date.into();
                l.ts = now + i as i64;
                s.put(&new_id(), l)?;
            }
            Ok(())
        })?;
        Ok(n)
    }

    /// Your usual foods: up to 8, ranked by how often and how recently you logged them.
    /// Each entry adds a score that halves every 14 days; amounts of the same food count
    /// together, and the chip repeats the most recent entry.
    pub fn recent_foods(&self) -> Vec<RecentFood> {
        let today = self.today();
        let base = |name: &str| food::base_name(name).to_lowercase();
        let mut score: HashMap<String, f64> = HashMap::new();
        let mut latest: HashMap<String, &Log> = HashMap::new();
        for (_, l) in self.store.all::<Log>() {
            let age = dates::days_between(&l.date, &today).max(0) as f64;
            if age > 90.0 {
                continue;
            }
            let k = base(&l.name);
            *score.entry(k.clone()).or_default() += 0.5f64.powf(age / 14.0);
            let e = latest.entry(k).or_insert(l);
            if l.ts > e.ts {
                *e = l;
            }
        }
        let mut ranked: Vec<(f64, &Log)> = latest.into_iter().map(|(k, l)| (score[&k], l)).collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.ts.cmp(&a.1.ts)));
        ranked
            .into_iter()
            .take(8)
            .map(|(_, l)| RecentFood {
                name: l.name.clone(),
                grams: l.grams,
                kcal: l.kcal,
                prot: l.prot,
                est: l.est,
                fiber: l.fiber,
                nut: l.nut.clone(),
                food_ref: l.food_ref.clone(),
            })
            .collect()
    }

    // ---------- Add: estimates ----------

    pub fn prepare_estimate(&self, text: &str) -> Result<Pending> {
        let text: String = text.chars().take(2000).collect();
        if text.trim().is_empty() {
            return Err("Describe what you ate first.".into());
        }
        let hhmm = self.hhmm();
        let local = food::local_estimate(&text, &hhmm, &self.learned());
        let ai = (!local.unknown.is_empty()).then(|| {
            let prompt = ai::food_prompt(&self.store.settings(), &self.store.rules_text(), &local.unknown.join(", "));
            (self.ai_config(), AiRequest { prompt, image: None })
        });
        Ok(Pending { local, ai, hhmm })
    }

    pub fn prepare_photo(&self, bytes: Vec<u8>, ext: &str, note: &str) -> Result<Pending> {
        if bytes.is_empty() {
            return Err("That photo is empty.".into());
        }
        if bytes.len() > 15 * 1024 * 1024 {
            return Err("That photo is too large (over 15 MB).".into());
        }
        let hhmm = self.hhmm();
        let text =
            if note.trim().is_empty() { "the meal in this photo".to_string() } else { format!("the meal in this photo ({})", note.trim()) };
        let prompt = ai::food_prompt(&self.store.settings(), &self.store.rules_text(), &text);
        let local = LocalEstimate { items: vec![], unknown: vec!["the photo".into()], meal: meal_for_time(&hhmm).into() };
        Ok(Pending { local, ai: Some((self.ai_config(), AiRequest { prompt, image: Some(Image { bytes, ext: ext.into() }) })), hhmm })
    }

    /// Merge the AI answer (if any) with the local estimate and learn new foods.
    pub fn finish_estimate(&mut self, p: Pending, answer: Option<std::result::Result<AiAnswer, String>>) -> EstimateView {
        // Rows worked out on the Mac come from the built-in table or foods you've taught it.
        let local: Vec<DraftItem> = p.local.items.iter().map(|it| self.local_draft(it)).collect();
        let mut view = EstimateView {
            items: local,
            unknown: p.local.unknown.clone(),
            note: String::new(),
            provider: None,
            provider_text: None,
            error: None,
            meal: p.local.meal.clone(),
        };
        match answer {
            None => {}
            Some(Err(e)) => view.error = Some(e),
            Some(Ok(ans)) => {
                let (items, note) = ai::clean_food(&ans.value, &p.hhmm);
                if items.is_empty() {
                    view.error = Some(format!("{} couldn't place that food.", ans.provider.label()));
                } else {
                    // Only foods with no database match are learned (their numbers are the AI's).
                    let rows: Vec<DraftItem> = items.into_iter().map(|it| self.match_draft(it)).collect();
                    let unmatched: Vec<Item> = rows.iter().filter(|r| r.item.food_ref.is_none()).map(|r| r.item.clone()).collect();
                    self.learn(&unmatched, ans.provider.id());
                    view.items.extend(rows);
                    view.unknown.clear();
                    view.note = note;
                    view.provider = Some(ans.provider.id().into());
                    view.provider_text = Some(worked_it_out(ans.provider));
                }
            }
        }
        view
    }

    /// A row from the local estimate, linked to its table or learned food when there is one.
    fn local_draft(&self, it: &Item) -> DraftItem {
        let base = food::base_name(&it.name).to_lowercase();
        let reference = std::iter::once(format!("table:{base}"))
            .chain(self.store.all::<Food>().filter(|(_, f)| f.name.eq_ignore_ascii_case(&base)).map(|(id, _)| format!("mine:{id}")))
            .find(|r| self.builtin_food(r).is_some());
        match reference.and_then(|r| Some((self.builtin_food(&r)?, r))) {
            Some((f, r)) => DraftItem {
                item: Item { food_ref: Some(r), ..it.clone() },
                source_name: Some(f.source_name),
                alts: vec![],
                portions: f.portions,
            },
            None => DraftItem { item: it.clone(), source_name: None, alts: vec![], portions: vec![] },
        }
    }

    /// Look an AI item up in the databases by the name the AI gave. A match replaces the
    /// AI's numbers with the database's for the same grams; no match keeps the AI estimate.
    fn match_draft(&self, it: Item) -> DraftItem {
        let fallback = |it: Item| DraftItem { item: it, source_name: None, alts: vec![], portions: vec![] };
        let (Some(q), Some(grams)) = (it.query.clone(), it.grams) else { return fallback(it) };
        // "chicken breast, roasted" → try all words, then drop words from the end until something matches.
        let words: Vec<&str> = q.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
        let mut hits = vec![];
        for n in (1..=words.len()).rev() {
            // A generic name never matches a packaged product ("apple" isn't "Apple juice (Coop)").
            hits = self.search_foods(&words[..n].join(" "), 8);
            hits.retain(|h| !crate::off::is_product(&h.source));
            if !hits.is_empty() {
                break;
            }
        }
        let Some(best) = hits.first() else { return fallback(it) };
        let Ok(mut row) = self.draft_item(&best.reference, grams, &it.meal) else { return fallback(it) };
        row.item.est = it.est;
        row.alts = hits
            .iter()
            .map(|h| FoodChoice { reference: h.reference.clone(), name: h.name.clone(), source_name: h.source_name.clone() })
            .collect();
        row
    }

    /// A draft row for a database food and amount: numbers from the database, never the AI.
    pub fn draft_item(&self, reference: &str, grams: f64, meal: &str) -> Result<DraftItem> {
        if !(grams > 0.0 && grams <= 5000.0) {
            return Err("Grams must be between 1 and 5000.".into());
        }
        let f = self.food_detail(reference).ok_or("That food isn't in your databases any more.")?;
        let k = grams / 100.0;
        let mut nut = nutrients::scale(&f.nut100, k);
        let fiber = nut.remove("fiber").map(round1);
        Ok(DraftItem {
            item: Item {
                name: format!("{} · {} g", f.name, food::fmt_num(grams.round())),
                grams: Some(grams.round()),
                kcal: (f.kcal100 * k).round(),
                prot: round1(f.prot100 * k),
                est: false,
                meal: valid_meal(meal, &self.hhmm()),
                fiber,
                nut: (!nut.is_empty()).then_some(nut),
                food_ref: Some(f.reference.clone()),
                query: None,
            },
            source_name: Some(f.source_name),
            alts: vec![],
            portions: f.portions,
        })
    }

    /// Change the amount of a row that has no database food (an AI estimate): everything scales.
    pub fn rescale_item(&self, item: Item, grams: f64) -> Result<Item> {
        if !(grams > 0.0 && grams <= 5000.0) {
            return Err("Grams must be between 1 and 5000.".into());
        }
        let old = item.grams.filter(|g| *g > 0.0).ok_or("This row has no weight to scale from.")?;
        let f = grams / old;
        let name = match item.name.rsplit_once(" · ") {
            Some((base, tail)) if tail.ends_with(" g") => format!("{base} · {} g", food::fmt_num(grams.round())),
            _ => item.name.clone(),
        };
        Ok(Item {
            name,
            grams: Some(grams.round()),
            kcal: (item.kcal * f).round(),
            prot: round1(item.prot * f),
            fiber: item.fiber.map(|x| round1(x * f)),
            nut: item.nut.as_ref().map(|n| nutrients::scale(n, f)),
            ..item
        })
    }

    fn learn(&mut self, items: &[Item], src: &str) {
        let now = self.now();
        for it in items {
            if let Some(f) = food::learn_from(it, src, now) {
                let existing = self.store.all::<Food>().find(|(_, x)| x.name.eq_ignore_ascii_case(&f.name)).map(|(id, _)| id.clone());
                let _ = self.store.put(&existing.unwrap_or_else(new_id), f);
            }
        }
    }

    // ---------- Food database ----------

    fn builtin_food(&self, reference: &str) -> Option<crate::fooddb::DbFood> {
        use crate::fooddb::{DbFood, Portion};
        let (kind, key) = reference.split_once(':')?;
        let (name, kcal, prot, piece, fiber, nut, source_name) = match kind {
            "table" => {
                let (n, k, p) = crate::food::TABLE.iter().find(|(n, _, _)| *n == key)?;
                let f = crate::food::match_table(&crate::food::words(n))?;
                (food::capitalize(n), *k, *p, f.piece, f.fiber100, f.nut100.clone().unwrap_or_default(), "Built-in table")
            }
            "mine" => {
                let f = self.store.get::<Food>(key)?;
                (f.name.clone(), f.kcal100, f.prot100, f.piece, f.fiber100, f.nut100.clone().unwrap_or_default(), "Learned foods")
            }
            "recipe" => {
                let r = self.store.get::<Recipe>(key)?;
                let v = recipe_view(key, r);
                let w = v.total.grams;
                if w <= 0.0 {
                    return None;
                }
                let k = 100.0 / w;
                let mut nut = recipe_nut(r);
                nut.remove("fiber");
                return Some(DbFood {
                    reference: reference.to_string(),
                    name: r.name.clone(),
                    source: "recipe".into(),
                    source_name: "My recipes".into(),
                    category: format!("{} serving{}", food::fmt_num(r.servings), if r.servings == 1.0 { "" } else { "s" }),
                    // Keep precision per 100 g so a serving adds back up exactly.
                    kcal100: (v.total.kcal * k * 1000.0).round() / 1000.0,
                    prot100: (v.total.prot * k * 1000.0).round() / 1000.0,
                    fiber100: v.total.fiber.map(|f| (f * k * 1000.0).round() / 1000.0),
                    known: v.known,
                    nut100: {
                        let mut n = nutrients::scale(&nut, k);
                        if let Some(f) = v.total.fiber {
                            n.insert("fiber".into(), (f * k * 1000.0).round() / 1000.0);
                        }
                        n
                    },
                    portions: vec![
                        Portion { label: "1 serving".into(), grams: v.per_serving.grams },
                        Portion { label: "whole recipe".into(), grams: w },
                    ],
                });
            }
            _ => return None,
        };
        let mut nut100 = nut;
        if let Some(f) = fiber {
            nut100.insert("fiber".into(), f);
        }
        Some(DbFood {
            reference: reference.to_string(),
            name,
            source: kind.to_string(),
            source_name: source_name.into(),
            category: String::new(),
            kcal100: kcal,
            prot100: prot,
            fiber100: fiber,
            known: nut100.len(),
            nut100,
            portions: if piece > 0.0 { vec![Portion { label: "1 piece".into(), grams: piece }] } else { vec![] },
        })
    }

    /// Search your built-in and learned foods, then the national databases.
    pub fn search_foods(&self, query: &str, limit: usize) -> Vec<crate::fooddb::DbFood> {
        let q: Vec<String> =
            query.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(String::from).collect();
        if q.is_empty() {
            return vec![];
        }
        let hit = |name: &str| {
            let words: Vec<String> =
                name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(String::from).collect();
            q.iter().all(|w| words.iter().any(|n| n.starts_with(w.as_str())))
        };
        let mut out: Vec<crate::fooddb::DbFood> = vec![];
        for (id, r) in self.store.all::<Recipe>() {
            if hit(&r.name) {
                out.extend(self.builtin_food(&format!("recipe:{id}")));
            }
        }
        for (id, f) in self.store.all::<Food>() {
            if hit(&f.name) {
                out.extend(self.builtin_food(&format!("mine:{id}")));
            }
        }
        for (n, _, _) in crate::food::TABLE {
            if hit(n) {
                out.extend(self.builtin_food(&format!("table:{n}")));
            }
        }
        out.truncate(6);
        if let Some(db) = &self.foods {
            out.extend(db.search(query, limit));
        }
        out.truncate(limit.clamp(1, 100));
        out
    }

    pub fn food_detail(&self, reference: &str) -> Option<crate::fooddb::DbFood> {
        self.builtin_food(reference).or_else(|| self.foods.as_ref()?.get(reference))
    }

    // ---------- Recipes ----------

    /// An ingredient from any food source, with its nutrition for `grams`.
    pub fn ingredient(&self, reference: &str, grams: f64) -> Result<Ingredient> {
        if !(grams > 0.0 && grams <= 20000.0) {
            return Err("Grams must be between 1 and 20000.".into());
        }
        let f = self.food_detail(reference).ok_or("That food isn't in your databases any more.")?;
        let k = grams / 100.0;
        let mut nut = nutrients::scale(&f.nut100, k);
        let fiber = nut.remove("fiber");
        Ok(Ingredient {
            name: f.name,
            food_ref: Some(f.reference),
            grams: grams.round(),
            kcal: (f.kcal100 * k).round(),
            prot: round1(f.prot100 * k),
            fiber,
            nut: (!nut.is_empty()).then_some(nut),
        })
    }

    fn clean_recipe(input: RecipeInput) -> Result<Recipe> {
        let name = ai::clip(&input.name, 80);
        if name.is_empty() {
            return Err("Give the recipe a name.".into());
        }
        if !(input.servings > 0.0 && input.servings <= 100.0) {
            return Err("Servings must be between 1 and 100.".into());
        }
        if input.ingredients.is_empty() {
            return Err("Add at least one ingredient.".into());
        }
        let cooked = input.cooked_grams.filter(|g| *g > 0.0 && g.is_finite());
        if cooked.is_some_and(|g| g > 50000.0) {
            return Err("The cooked weight looks too large.".into());
        }
        let ingredients = input
            .ingredients
            .into_iter()
            .take(60)
            .map(|i| Ingredient {
                name: ai::clip(&i.name, 80),
                food_ref: i.food_ref,
                grams: i.grams.clamp(0.0, 20000.0),
                kcal: i.kcal.clamp(0.0, 50000.0).round(),
                prot: round1(i.prot.clamp(0.0, 5000.0)),
                fiber: i.fiber.filter(|f| f.is_finite() && *f >= 0.0),
                nut: i.nut.as_ref().map(nutrients::clean).filter(|n| !n.is_empty()),
            })
            .filter(|i| !i.name.is_empty())
            .collect();
        Ok(Recipe { name, servings: input.servings, cooked_grams: cooked, ingredients, note: ai::clip(&input.note, 500) })
    }

    /// Totals for a recipe being edited, without saving it.
    pub fn recipe_preview(&self, input: RecipeInput) -> Result<RecipeView> {
        let r = Self::clean_recipe(input)?;
        Ok(recipe_view("", &r))
    }

    pub fn save_recipe(&mut self, id: Option<String>, input: RecipeInput) -> Result<String> {
        let r = Self::clean_recipe(input)?;
        let id = id.filter(|i| !i.is_empty()).unwrap_or_else(new_id);
        self.store.put(&id, r)?;
        Ok(id)
    }

    pub fn recipes(&self) -> Vec<RecipeView> {
        let mut v: Vec<RecipeView> = self.store.all::<Recipe>().map(|(id, r)| recipe_view(id, r)).collect();
        v.sort_by_key(|r| r.name.to_lowercase());
        v
    }

    pub fn delete_recipe(&mut self, id: &str) -> Result<()> {
        self.store.delete::<Recipe>(id).map(|_| ())
    }

    pub fn food_sources(&self) -> Vec<crate::fooddb::SourceInfo> {
        self.foods.as_ref().map(|d| d.sources()).unwrap_or_default()
    }

    /// Log `grams` of a database food, with everything the database knows about it.
    pub fn add_food(&mut self, date: &str, meal: &str, reference: &str, grams: f64) -> Result<String> {
        check_date(date)?;
        if !(grams > 0.0 && grams <= 5000.0) {
            return Err("Grams must be between 1 and 5000.".into());
        }
        let f = self.food_detail(reference).ok_or("That food isn't in your databases any more.")?;
        let k = grams / 100.0;
        let mut nut = nutrients::scale(&f.nut100, k);
        let fiber = nut.remove("fiber");
        let id = new_id();
        let log = Log {
            date: date.into(),
            meal: valid_meal(meal, &self.hhmm()),
            name: format!("{} · {} g", f.name, food::fmt_num(grams.round())),
            grams: Some(grams.round()),
            kcal: (f.kcal100 * k).round(),
            prot: round1(f.prot100 * k),
            est: false,
            ts: self.now(),
            fiber,
            nut: (!nut.is_empty()).then_some(nut),
            food_ref: Some(f.reference),
        };
        if log.kcal >= 5000.0 {
            return Err("That's more than 5000 kcal in one entry.".into());
        }
        self.store.put(&id, log)?;
        Ok(id)
    }

    /// Import the Swiss database from the official xlsx (downloaded by the app).
    #[cfg(feature = "import")]
    pub fn import_swiss(&mut self, bytes: &[u8], version: &str) -> Result<usize> {
        let db = self.foods.as_ref().ok_or("The food database isn't open.")?;
        let foods = crate::importers::swiss_from_xlsx(bytes)?;
        db.replace_source(&crate::importers::swiss_info(version), &foods)
    }

    /// Save products fetched from Open Food Facts and return them as foods.
    /// Products already in the Swiss pack are returned from the pack, not saved twice.
    pub fn save_off(&mut self, foods: Vec<crate::fooddb::NewFood>) -> Result<Vec<crate::fooddb::DbFood>> {
        let db = self.foods.as_ref().ok_or("The food database isn't open.")?;
        let in_pack = |f: &crate::fooddb::NewFood| db.get(&format!("{}:{}", crate::off::PACK_SOURCE, f.source_id));
        let new: Vec<_> = foods.iter().filter(|f| in_pack(f).is_none()).cloned().collect();
        db.add_foods(&crate::off::info(), &new)?;
        Ok(foods.iter().filter_map(|f| in_pack(f).or_else(|| db.get(&format!("off:{}", f.source_id)))).collect())
    }

    /// Swap in a freshly built Swiss pack file (`tmp`, from `off::download_pack`) as `dest`.
    pub fn install_off_pack(&mut self, tmp: &std::path::Path, dest: &std::path::Path) -> Result<()> {
        let db = self.foods.as_mut().ok_or("The food database isn't open.")?;
        db.detach(crate::off::PACK_SOURCE);
        if let Err(e) = std::fs::rename(tmp, dest) {
            let _ = std::fs::remove_file(tmp);
            if dest.is_file() {
                let _ = db.attach(dest);
            }
            return Err(format!("Couldn't save the Swiss products: {e}"));
        }
        db.attach(dest)
    }

    pub fn remove_food_source(&mut self, id: &str) -> Result<()> {
        if id == "usda" || id == "cofid" {
            return Err("That database comes with the app and can't be removed.".into());
        }
        let db = self.foods.as_mut().ok_or("The food database isn't open.")?;
        if let Some(path) = db.detach(id) {
            return std::fs::remove_file(&path).map_err(|e| format!("Couldn't delete {}: {e}", path.display()));
        }
        db.remove_source(id)
    }

    pub fn quick(&self, name: &str, grams: Option<f64>) -> Option<QuickFill> {
        let learned = self.learned();
        let f = food::match_food(&food::words(name), &learned)?;
        let (kcal, prot, grams) = food::quick(name, grams, &learned)?;
        let (fiber, _) = food::extras(&f, grams);
        Some(QuickFill { kcal, prot, fiber, grams, matched: food::capitalize(&f.name) })
    }

    pub fn food_names(&self) -> Vec<String> {
        let mut v = food::names();
        v.extend(self.store.all::<Food>().map(|(_, f)| f.name.clone()));
        v.sort_by_key(|a| a.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    pub fn learned_foods(&self) -> Vec<LearnedFood> {
        let mut v: Vec<LearnedFood> = self
            .store
            .all::<Food>()
            .map(|(id, f)| LearnedFood {
                id: id.clone(),
                name: f.name.clone(),
                kcal100: f.kcal100,
                prot100: f.prot100,
                piece: f.piece,
                src: f.src.clone(),
            })
            .collect();
        v.sort_by_key(|a| a.name.to_lowercase());
        v
    }

    pub fn delete_learned(&mut self, id: &str) -> Result<()> {
        self.store.delete::<Food>(id).map(|_| ())
    }

    // ---------- Meds ----------

    /// The record of a group that applies today (or the latest one if all ended).
    fn current_med(&self, group: &str) -> Option<(String, Med)> {
        let today = self.today();
        let mut recs: Vec<(&String, &Med)> = self.store.all::<Med>().filter(|(_, m)| m.group_id == group).collect();
        recs.sort_by(|a, b| a.1.from.cmp(&b.1.from));
        recs.iter()
            .rev()
            .find(|(_, m)| m.until.as_deref().is_none_or(|u| dates::days_between(&today, u) >= 0))
            .map(|(id, m)| ((*id).clone(), (*m).clone()))
    }

    pub fn meds_view(&self) -> MedsView {
        let today = self.today();
        let groups: BTreeMap<String, ()> = self.store.all::<Med>().map(|(_, m)| (m.group_id.clone(), ())).collect();
        let mut v = MedsView { daily: vec![], days: vec![], monthly: vec![], stopped: vec![] };
        for g in groups.keys() {
            let to_view = |id: &str, m: &Med| MedView {
                id: id.into(),
                group_id: m.group_id.clone(),
                name: m.name.clone(),
                dose: m.dose.clone(),
                freq: m.freq.clone(),
                days: m.days.clone(),
                dom: m.dom,
                time: m.time.clone(),
                from: m.from.clone(),
                until: m.until.clone(),
            };
            match self.current_med(g) {
                Some((id, m)) => {
                    let mv = to_view(&id, &m);
                    match m.freq.as_str() {
                        "weekly" => v.days.push(mv),
                        "monthly" => v.monthly.push(mv),
                        _ => v.daily.push(mv),
                    }
                }
                None => {
                    let last = self.store.all::<Med>().filter(|(_, m)| &m.group_id == g).max_by(|a, b| a.1.from.cmp(&b.1.from));
                    if let Some((id, m)) = last {
                        v.stopped.push(to_view(id, m));
                    }
                }
            }
        }
        let key = |m: &MedView| (m.time.is_empty(), m.time.clone(), m.name.to_lowercase());
        for list in [&mut v.daily, &mut v.days, &mut v.monthly] {
            list.sort_by_key(key);
        }
        v.stopped.sort_by_key(|m| m.name.to_lowercase());
        let _ = today;
        v
    }

    /// Save a medicine. A schedule change ends the current record yesterday and starts
    /// a new one today (same group); if the current record only started today, it is replaced.
    pub fn save_med(&mut self, input: MedInput) -> Result<SaveMedResult> {
        let today = self.today();
        let yesterday = dates::add_days(&today, -1);
        let mut m = Med {
            group_id: String::new(),
            name: input.name,
            dose: input.dose,
            freq: input.freq,
            days: input.days,
            dom: input.dom,
            time: input.time,
            from: today.clone(),
            until: None,
            active: true,
        };
        meds::validate(&mut m)?;
        let Some(group) = input.group_id.filter(|g| !g.is_empty()) else {
            let g = new_id();
            m.group_id = g.clone();
            self.store.put(&new_id(), m)?;
            return Ok(SaveMedResult { group_id: g, started_new: false });
        };
        m.group_id = group.clone();
        let Some((cur_id, mut cur)) = self.current_med(&group) else {
            // Restarting a medicine that was stopped.
            self.store.put(&new_id(), m)?;
            return Ok(SaveMedResult { group_id: group, started_new: true });
        };
        if !meds::schedule_changed(&cur, &m) {
            cur.name = m.name;
            cur.dose = m.dose;
            self.store.put(&cur_id, cur)?;
            return Ok(SaveMedResult { group_id: group, started_new: false });
        }
        if dates::days_between(&today, &cur.from) >= 0 {
            m.from = cur.from.clone();
            self.store.put(&cur_id, m)?;
            return Ok(SaveMedResult { group_id: group, started_new: false });
        }
        cur.until = Some(yesterday);
        self.store.put(&cur_id, cur)?;
        let new = new_id();
        self.store.put(&new, m)?;
        // Today's tick moves to the new record.
        let old_tick = format!("{today}_{cur_id}");
        if let Some(mut t) = self.store.get::<MedLog>(&old_tick).cloned() {
            self.store.delete::<MedLog>(&old_tick)?;
            t.med_id = new.clone();
            self.store.put(&format!("{today}_{new}"), t)?;
        }
        Ok(SaveMedResult { group_id: group, started_new: true })
    }

    /// "Stop taking": the current record ends yesterday.
    pub fn stop_med(&mut self, group: &str) -> Result<()> {
        let yesterday = dates::add_days(&self.today(), -1);
        let (id, mut m) = self.current_med(group).ok_or("That medicine is already stopped.")?;
        m.until = Some(yesterday);
        m.active = false;
        self.store.put(&id, m)
    }

    /// Erase a medicine group and all its ticks.
    pub fn erase_med(&mut self, group: &str) -> Result<usize> {
        let ids: HashSet<String> = self.store.all::<Med>().filter(|(_, m)| m.group_id == group).map(|(id, _)| id.clone()).collect();
        let ticks: Vec<String> = self.store.all::<MedLog>().filter(|(_, t)| ids.contains(&t.med_id)).map(|(id, _)| id.clone()).collect();
        self.store.batch(|s| {
            for t in &ticks {
                s.delete::<MedLog>(t)?;
            }
            for id in &ids {
                s.delete::<Med>(id)?;
            }
            Ok(ids.len())
        })
    }

    /// Tick or untick a dose. Returns whether it is now taken.
    pub fn toggle_dose(&mut self, date: &str, med_id: &str) -> Result<bool> {
        check_date(date)?;
        if dates::days_between(&self.today(), date) > 0 {
            return Err("You can't tick a dose in the future.".into());
        }
        let m = self.store.get::<Med>(med_id).ok_or("That medicine no longer exists.")?;
        if !meds::is_due(m, date) {
            return Err("That medicine isn't due on this day.".into());
        }
        let id = format!("{date}_{med_id}");
        if self.store.get::<MedLog>(&id).is_some() {
            self.store.delete::<MedLog>(&id)?;
            Ok(false)
        } else {
            let at = if date == self.today() { self.hhmm() } else { String::new() };
            self.store.put(&id, MedLog { date: date.into(), med_id: med_id.into(), at, ts: self.now() })?;
            Ok(true)
        }
    }

    // ---------- Training ----------

    pub fn burn_estimate(&self, kind: &str, mins: f64) -> f64 {
        training::burn(kind, mins, self.store.settings().weight_kg)
    }

    pub fn add_workout(&mut self, date: &str, kind: &str, mins: f64, kcal: Option<f64>, name: &str) -> Result<String> {
        check_date(date)?;
        if dates::days_between(&self.today(), date) > 0 {
            return Err("You can't log training in the future.".into());
        }
        if !(mins > 0.0 && mins <= 1440.0) {
            return Err("Minutes must be between 1 and 1440.".into());
        }
        let kind = if KINDS.contains(&kind) { kind } else { "other" };
        let kcal = match kcal.filter(|k| *k > 0.0) {
            Some(k) if k > training::MAX_SESSION_KCAL => return Err("That's more than 5000 kcal for one session.".into()),
            Some(k) => k.round(),
            None => self.burn_estimate(kind, mins),
        };
        let name = ai::clip(name, 80);
        let id = new_id();
        let w = Workout {
            date: date.into(),
            kind: kind.into(),
            mins: mins.round(),
            kcal,
            name: if name.is_empty() { food::capitalize(kind) } else { name },
            src: "manual".into(),
            ts: self.now(),
        };
        self.store.put(&id, w)?;
        Ok(id)
    }

    /// The most recent training sessions, newest first.
    pub fn recent_workouts(&self, limit: usize) -> Vec<WorkoutEntry> {
        let mut v: Vec<(&String, &Workout)> = self.store.all::<Workout>().collect();
        v.sort_by(|a, b| (&b.1.date, b.1.ts).cmp(&(&a.1.date, a.1.ts)));
        v.into_iter()
            .take(limit.min(500))
            .map(|(id, w)| WorkoutEntry {
                id: id.clone(),
                date: w.date.clone(),
                kind: w.kind.clone(),
                mins: w.mins,
                kcal: w.kcal,
                name: w.name.clone(),
            })
            .collect()
    }

    /// Edit a training session (date stays).
    pub fn update_workout(&mut self, id: &str, kind: &str, mins: f64, kcal: Option<f64>, name: &str) -> Result<()> {
        let mut w = self.store.get::<Workout>(id).cloned().ok_or("That session no longer exists.")?;
        if !(mins > 0.0 && mins <= 1440.0) {
            return Err("Minutes must be between 1 and 1440.".into());
        }
        let kind = if KINDS.contains(&kind) { kind } else { "other" };
        w.kcal = match kcal.filter(|k| *k > 0.0) {
            Some(k) if k > training::MAX_SESSION_KCAL => return Err("That's more than 5000 kcal for one session.".into()),
            Some(k) => k.round(),
            None => self.burn_estimate(kind, mins),
        };
        w.kind = kind.into();
        w.mins = mins.round();
        let n = ai::clip(name, 80);
        w.name = if n.is_empty() { food::capitalize(kind) } else { n };
        self.store.put(id, w)
    }

    pub fn delete_workout(&mut self, id: &str) -> Result<()> {
        self.store.delete::<Workout>(id).map(|_| ())
    }

    pub fn import_workouts(&mut self, text: &str) -> Result<ImportResult> {
        let r = training::parse_paste(text, &self.today(), self.store.settings().weight_kg);
        let now = self.reserve_ts(r.sessions.len());
        let added = r.sessions.len();
        self.store.batch(|s| {
            for (i, x) in r.sessions.into_iter().enumerate() {
                let w = Workout {
                    date: x.date,
                    kind: x.kind,
                    mins: x.mins,
                    kcal: x.kcal,
                    name: x.name,
                    src: "import".into(),
                    ts: now + i as i64,
                };
                s.put(&new_id(), w)?;
            }
            Ok(())
        })?;
        Ok(ImportResult { added, skipped: r.skipped })
    }

    pub fn set_add_burn(&mut self, on: bool) -> Result<()> {
        let mut s = self.store.settings();
        s.add_burn = on;
        self.store.put("main", s)
    }

    // ---------- Stock ----------

    pub fn stock_view(&self) -> StockView {
        let today = self.today();
        let mut by_cat: BTreeMap<usize, Vec<StockRow>> = BTreeMap::new();
        let mut soon_count = 0;
        let mut total = 0;
        for (id, s) in self.store.all::<Stock>() {
            let idx = STOCK_CATS.iter().position(|c| *c == s.cat).unwrap_or(3);
            let age = if dates::is_date(&s.added) { dates::days_between(&s.added, &today) } else { 0 };
            let left = s.use_by.as_deref().filter(|u| dates::is_date(u)).map(|u| dates::days_between(&today, u));
            let soon = left.is_some_and(|d| d <= 7);
            soon_count += soon as usize;
            total += 1;
            by_cat.entry(idx).or_default().push(StockRow {
                id: id.clone(),
                name: s.name.clone(),
                qty: s.qty,
                unit: s.unit.clone(),
                cat: STOCK_CATS[idx].into(),
                added: s.added.clone(),
                use_by: s.use_by.clone(),
                age_days: age,
                days_left: left,
                soon,
                eat_today: STOCK_CATS[idx] == "cooked" && age >= 3,
            });
        }
        let sections = by_cat
            .into_iter()
            .map(|(i, mut items)| {
                items.sort_by(|a, b| {
                    (a.days_left.unwrap_or(i64::MAX), a.name.to_lowercase()).cmp(&(b.days_left.unwrap_or(i64::MAX), b.name.to_lowercase()))
                });
                StockSection { cat: STOCK_CATS[i].into(), items }
            })
            .collect();
        StockView { sections, soon_count, total }
    }

    pub fn save_stock(&mut self, id: Option<String>, mut s: Stock) -> Result<String> {
        s.name = ai::clip(&s.name, 80);
        if s.name.is_empty() {
            return Err("Give the item a name.".into());
        }
        if !(s.qty.is_finite() && s.qty >= 0.0 && s.qty < 1_000_000.0) {
            return Err("The amount isn't valid.".into());
        }
        s.unit = ai::clip(&s.unit, 20);
        if !STOCK_CATS.contains(&s.cat.as_str()) {
            s.cat = "shelf".into();
        }
        if !dates::is_date(&s.added) {
            s.added = self.today();
        }
        s.use_by = s.use_by.filter(|u| dates::is_date(u));
        let id = id.filter(|i| !i.is_empty()).unwrap_or_else(new_id);
        self.store.put(&id, s)?;
        Ok(id)
    }

    pub fn delete_stock(&mut self, id: &str) -> Result<()> {
        self.store.delete::<Stock>(id).map(|_| ())
    }

    // ---------- Shopping ----------

    fn list_row(id: &str, l: &ListItem) -> ListRow {
        ListRow {
            id: id.into(),
            name: l.name.clone(),
            qty: l.qty.clone(),
            cat: l.cat.clone(),
            price: l.price,
            trip: l.trip,
            checked: l.checked,
        }
    }

    pub fn shop_view(&self, trip: u8) -> ShopView {
        let trip = if trip == 2 { 2 } else { 1 };
        let mut groups: BTreeMap<usize, Vec<ListRow>> = BTreeMap::new();
        let mut in_stock = vec![];
        let mut spend = 0.0;
        let mut trip_spend = 0.0;
        let mut checked = 0;
        for (id, l) in self.store.all::<ListItem>() {
            if l.in_stock {
                if l.trip == trip {
                    in_stock.push(Self::list_row(id, l));
                }
                continue;
            }
            let p = l.price.unwrap_or(0.0);
            spend += p;
            if l.trip != trip {
                continue;
            }
            trip_spend += p;
            checked += l.checked as usize;
            let idx = STOCK_CATS.iter().position(|c| *c == l.cat).unwrap_or(3);
            groups.entry(idx).or_default().push(Self::list_row(id, l));
        }
        let groups = groups
            .into_iter()
            .map(|(i, mut items)| {
                items.sort_by_key(|x| x.name.to_lowercase());
                ListGroup { cat: STOCK_CATS[i].into(), items }
            })
            .collect();
        in_stock.sort_by_key(|x| x.name.to_lowercase());
        let spend = (spend * 100.0).round() / 100.0;
        ShopView {
            trip,
            budget: BUDGET_CHF,
            spend,
            trip_spend: (trip_spend * 100.0).round() / 100.0,
            over_budget: spend > BUDGET_CHF,
            groups,
            in_stock,
            checked_count: checked,
        }
    }

    pub fn save_list_item(&mut self, id: Option<String>, mut l: ListItem) -> Result<String> {
        l.name = ai::clip(&l.name, 80);
        if l.name.is_empty() {
            return Err("Give the item a name.".into());
        }
        l.qty = ai::clip(&l.qty, 30);
        if !STOCK_CATS.contains(&l.cat.as_str()) {
            l.cat = "shelf".into();
        }
        l.trip = if l.trip == 2 { 2 } else { 1 };
        if let Some(p) = l.price {
            if !(p.is_finite() && (0.0..10_000.0).contains(&p)) {
                return Err("The price isn't valid.".into());
            }
        }
        let id = id.filter(|i| !i.is_empty()).unwrap_or_else(new_id);
        if let Some(old) = self.store.get::<ListItem>(&id) {
            l.in_stock = old.in_stock;
        }
        self.store.put(&id, l)?;
        Ok(id)
    }

    pub fn toggle_list_item(&mut self, id: &str) -> Result<bool> {
        let mut l = self.store.get::<ListItem>(id).cloned().ok_or("That item is gone.")?;
        l.checked = !l.checked;
        let c = l.checked;
        self.store.put(id, l)?;
        Ok(c)
    }

    pub fn delete_list_item(&mut self, id: &str) -> Result<()> {
        self.store.delete::<ListItem>(id).map(|_| ())
    }

    /// List items that look like something already in stock (either name contains the other).
    pub fn check_have(&self) -> Vec<HaveMatch> {
        let stock: Vec<(&String, &Stock)> = self.store.all::<Stock>().collect();
        let mut out = vec![];
        let mut items: Vec<(&String, &ListItem)> = self.store.all::<ListItem>().filter(|(_, l)| !l.in_stock).collect();
        items.sort_by_key(|(_, l)| l.name.to_lowercase());
        for (lid, l) in items {
            let ln = l.name.trim().to_lowercase();
            if ln.is_empty() {
                continue;
            }
            if let Some((sid, s)) = stock.iter().find(|(_, s)| {
                let sn = s.name.trim().to_lowercase();
                !sn.is_empty() && (ln.contains(&sn) || sn.contains(&ln))
            }) {
                out.push(HaveMatch {
                    list_id: lid.clone(),
                    list_name: l.name.clone(),
                    stock_id: (*sid).clone(),
                    stock_name: s.name.clone(),
                    stock_qty: s.qty,
                    stock_unit: s.unit.clone(),
                });
            }
        }
        out
    }

    /// Still there → the list item moves to "already in stock". Used up → the stock row goes.
    pub fn resolve_have(&mut self, list_id: &str, stock_id: &str, still_there: bool) -> Result<()> {
        if still_there {
            let mut l = self.store.get::<ListItem>(list_id).cloned().ok_or("That item is gone.")?;
            l.in_stock = true;
            l.checked = false;
            self.store.put(list_id, l)
        } else {
            self.store.delete::<Stock>(stock_id).map(|_| ())
        }
    }

    pub fn move_back(&mut self, list_id: &str) -> Result<()> {
        let mut l = self.store.get::<ListItem>(list_id).cloned().ok_or("That item is gone.")?;
        l.in_stock = false;
        self.store.put(list_id, l)
    }

    /// "Bought it": ticked items of a trip go into stock, merging same name + unit.
    pub fn bought(&mut self, trip: u8) -> Result<usize> {
        let today = self.today();
        let items: Vec<(String, ListItem)> = self
            .store
            .all::<ListItem>()
            .filter(|(_, l)| l.checked && !l.in_stock && l.trip == trip)
            .map(|(id, l)| (id.clone(), l.clone()))
            .collect();
        self.store.batch(|s| {
            for (lid, l) in &items {
                let (qty, unit) = parse_qty(&l.qty);
                let existing = s
                    .all::<Stock>()
                    .find(|(_, x)| x.name.trim().eq_ignore_ascii_case(l.name.trim()) && x.unit.trim().eq_ignore_ascii_case(unit.trim()))
                    .map(|(id, x)| (id.clone(), x.clone()));
                match existing {
                    Some((sid, mut x)) => {
                        x.qty += qty;
                        s.put(&sid, x)?;
                    }
                    None => {
                        let cat = if STOCK_CATS.contains(&l.cat.as_str()) { l.cat.clone() } else { "shelf".into() };
                        s.put(&new_id(), Stock { name: l.name.trim().into(), qty, unit, cat, added: today.clone(), use_by: None })?;
                    }
                }
                s.delete::<ListItem>(lid)?;
            }
            Ok(items.len())
        })
    }

    // ---------- Week ----------

    pub fn week_view(&self) -> WeekView {
        let today = self.today();
        let start = dates::add_days(&today, -7);
        let last_plan =
            self.store.all::<Plan>().filter(|(_, p)| !p.meals.is_empty()).map(|(d, _)| d.clone()).filter(|d| dates::is_date(d)).max();
        let mut end = dates::add_days(&today, 7);
        if let Some(l) = last_plan {
            let cap = dates::add_days(&today, 60);
            if l > end {
                end = if l > cap { cap } else { l };
            }
        }
        let mut days = vec![];
        let mut d = start;
        let (mut sk, mut sp, mut n) = (0.0, 0.0, 0);
        while d <= end {
            let meals = self.store.get::<Plan>(&d).map(|p| p.meals.clone()).unwrap_or_default();
            let kcal: f64 = meals.iter().map(|m| m.kcal).sum();
            let prot: f64 = meals.iter().map(|m| m.prot).sum();
            if !meals.is_empty() {
                sk += kcal;
                sp += prot;
                n += 1;
            }
            days.push(PlanDay {
                dow: dates::weekday(&d),
                is_today: d == today,
                date: d.clone(),
                meals,
                kcal: kcal.round(),
                prot: round1(prot),
            });
            d = dates::add_days(&d, 1);
        }
        WeekView {
            days,
            avg_kcal: if n > 0 { (sk / n as f64).round() } else { 0.0 },
            avg_prot: if n > 0 { round1(sp / n as f64) } else { 0.0 },
            planned_days: n,
        }
    }

    /// Today and the next `days - 1` days, with totals against your targets.
    pub fn plan_ahead(&self, days: usize) -> PlanAhead {
        let today = self.today();
        let s = self.store.settings();
        let mut out = vec![];
        let (mut sk, mut sp, mut n) = (0.0, 0.0, 0);
        for i in 0..days.clamp(1, 28) as i64 {
            let d = dates::add_days(&today, i);
            let meals: Vec<PlanAheadMeal> = self
                .store
                .get::<Plan>(&d)
                .map(|p| {
                    p.meals
                        .iter()
                        .enumerate()
                        .map(|(idx, m)| PlanAheadMeal {
                            idx,
                            slot: m.slot.clone(),
                            name: m.name.clone(),
                            kcal: m.kcal,
                            prot: m.prot,
                            fiber: m.fiber,
                            done: m.done,
                        })
                        .collect()
                })
                .unwrap_or_default();
            let kcal: f64 = meals.iter().map(|m| m.kcal).sum();
            let prot: f64 = meals.iter().map(|m| m.prot).sum();
            let fiber: f64 = meals.iter().filter_map(|m| m.fiber).sum();
            if !meals.is_empty() {
                sk += kcal;
                sp += prot;
                n += 1;
            }
            let empty_slots = MEALS.iter().filter(|slot| !meals.iter().any(|m| m.slot == **slot)).map(|s| s.to_string()).collect();
            let shop = self.shop_day(&d);
            out.push(PlanAheadDay {
                dow: dates::weekday(&d),
                is_today: i == 0,
                offset: i,
                over: kcal > s.kcal_target,
                prot_met: prot >= s.prot_floor,
                date: d,
                meals,
                kcal: kcal.round(),
                prot: round1(prot),
                fiber: round1(fiber),
                empty_slots,
                shop,
            });
        }
        let has_earlier = self.store.all::<Plan>().any(|(d, p)| !p.meals.is_empty() && dates::days_between(d, &today) > 0);
        PlanAhead {
            days: out,
            kcal_target: s.kcal_target,
            prot_floor: s.prot_floor,
            fiber_floor: s.fiber_floor,
            planned_days: n,
            avg_kcal: if n > 0 { (sk / n as f64).round() } else { 0.0 },
            avg_prot: if n > 0 { round1(sp / n as f64) } else { 0.0 },
            has_earlier,
        }
    }

    /// Mark `date` as the day you shop for `trip` (1 or 2), or clear it with None.
    pub fn set_shop_day(&mut self, date: &str, trip: Option<u8>) -> Result<()> {
        check_date(date)?;
        if trip.is_some_and(|t| t != 1 && t != 2) {
            return Err("Pick Trip 1 or Trip 2.".into());
        }
        let mut p = self.store.get::<Plan>(date).cloned().unwrap_or_default();
        if p.shop == trip {
            return Ok(());
        }
        p.shop = trip;
        self.store.put(date, p)
    }

    /// The shopping planned for `date`, with what's left on that trip's list.
    pub fn shop_day(&self, date: &str) -> Option<ShopDay> {
        let trip = self.store.get::<Plan>(date)?.shop?;
        let left: Vec<&ListItem> =
            self.store.all::<ListItem>().map(|(_, l)| l).filter(|l| l.trip == trip && !l.checked && !l.in_stock).collect();
        let cost = left.iter().filter_map(|l| l.price).sum::<f64>();
        Some(ShopDay { trip, items: left.len(), cost: (cost * 100.0).round() / 100.0 })
    }

    /// Plan a database food (or recipe) for a meal: like `add_food`, into the plan instead of the log.
    pub fn plan_food(&mut self, date: &str, slot: &str, reference: &str, grams: f64) -> Result<()> {
        check_date(date)?;
        if !(grams > 0.0 && grams <= 5000.0) {
            return Err("Grams must be between 1 and 5000.".into());
        }
        let f = self.food_detail(reference).ok_or("That food isn't in your databases any more.")?;
        let k = grams / 100.0;
        let kcal = (f.kcal100 * k).round();
        if kcal >= 5000.0 {
            return Err("That's more than 5000 kcal in one meal.".into());
        }
        let fiber = f.nut100.get("fiber").map(|x| round1(x * k));
        let meal = PlanMeal {
            slot: valid_meal(slot, "12:00"),
            name: format!("{} · {} g", f.name, food::fmt_num(grams.round())),
            kcal,
            prot: round1(f.prot100 * k),
            fiber,
            ..Default::default()
        };
        self.add_plan_meal(date, meal)
    }

    /// Add one meal to a day's plan.
    pub fn add_plan_meal(&mut self, date: &str, meal: PlanMeal) -> Result<()> {
        let mut meals = self.store.get::<Plan>(date).map(|p| p.meals.clone()).unwrap_or_default();
        meals.push(PlanMeal { done: false, log_id: None, ..meal });
        self.set_plan(date, meals)
    }

    /// Remove one meal from a day's plan (and its log row if it was ticked).
    pub fn remove_plan_meal(&mut self, date: &str, idx: usize, expect: Option<&str>) -> Result<()> {
        let mut meals = self.store.get::<Plan>(date).map(|p| p.meals.clone()).unwrap_or_default();
        plan_meal_at(&meals, idx, expect)?;
        meals.remove(idx);
        self.set_plan(date, meals)
    }

    /// This week's training at a glance.
    pub fn training_summary(&self) -> TrainingSummary {
        let today = self.today();
        let mut week = (0, 0.0, 0.0);
        let mut kinds: BTreeMap<String, f64> = BTreeMap::new();
        let mut daily: BTreeMap<String, f64> = BTreeMap::new();
        for i in 0..28 {
            daily.insert(dates::add_days(&today, -i), 0.0);
        }
        let mut last: Option<i64> = None;
        for (_, w) in self.store.all::<Workout>() {
            let ago = dates::days_between(&w.date, &today);
            if ago < 0 {
                continue;
            }
            last = Some(last.map_or(ago, |l: i64| l.min(ago)));
            if ago < 7 {
                week.0 += 1;
                week.1 += w.mins;
                week.2 += w.kcal;
            }
            if ago < 28 {
                *kinds.entry(w.kind.clone()).or_default() += w.mins;
                *daily.entry(w.date.clone()).or_default() += w.mins;
            }
        }
        let mut by_kind: Vec<(String, f64)> = kinds.into_iter().collect();
        by_kind.sort_by(|a, b| b.1.total_cmp(&a.1));
        TrainingSummary {
            week_sessions: week.0,
            week_mins: week.1,
            week_kcal: week.2.round(),
            last_days_ago: last,
            by_kind,
            daily: daily.into_iter().collect(),
        }
    }

    /// Copy the latest 7 planned days forward by 7 days, skipping planned days; ticks reset.
    pub fn repeat_last_week(&mut self) -> Result<usize> {
        let mut planned: Vec<(String, Plan)> = self
            .store
            .all::<Plan>()
            .filter(|(d, p)| !p.meals.is_empty() && dates::is_date(d))
            .map(|(d, p)| (d.clone(), p.clone()))
            .collect();
        if planned.is_empty() {
            return Err("There's no planned week to repeat yet.".into());
        }
        planned.sort_by(|a, b| b.0.cmp(&a.0));
        planned.truncate(7);
        let mut n = 0;
        for (d, p) in planned {
            let target = dates::add_days(&d, 7);
            if self.store.get::<Plan>(&target).is_some_and(|p| !p.meals.is_empty()) {
                continue;
            }
            let meals = p.meals.into_iter().map(|m| PlanMeal { done: false, log_id: None, ..m }).collect();
            // The shopping day repeats too, unless that day already has one.
            let shop = self.store.get::<Plan>(&target).and_then(|t| t.shop).or(p.shop);
            self.store.put(&target, Plan { meals, shop })?;
            n += 1;
        }
        Ok(n)
    }

    // ---------- History ----------

    pub fn history(&self) -> History {
        self.history_range(60)
    }

    /// The last `range` days (14, 30 or 60), trimmed to the first day used, with
    /// series for charts, per-medicine adherence and "what's missing" insights.
    pub fn history_range(&self, range: usize) -> History {
        let range = range.clamp(7, 60) as i64;
        let today = self.today();
        let start = dates::add_days(&today, 1 - range);
        let s = self.store.settings();
        #[derive(Default, Clone)]
        struct Acc {
            kcal: f64,
            prot: f64,
            fiber: f64,
            fiber_known: bool,
            nut: Nut,
        }
        let mut food: HashMap<&str, Acc> = HashMap::new();
        for (_, l) in self.store.all::<Log>() {
            let e = food.entry(l.date.as_str()).or_default();
            e.kcal += l.kcal;
            e.prot += l.prot;
            if let Some(f) = l.fiber {
                e.fiber += f;
                e.fiber_known = true;
            }
            nutrients::add(&mut e.nut, &row_nut(l));
        }
        let mut mins: HashMap<&str, f64> = HashMap::new();
        let mut burns: HashMap<&str, f64> = HashMap::new();
        for (_, w) in self.store.all::<Workout>() {
            *mins.entry(w.date.as_str()).or_default() += w.mins;
            *burns.entry(w.date.as_str()).or_default() += w.kcal;
        }
        let mut ticks: HashSet<(&str, &str)> = HashSet::new();
        let mut tick_days: HashSet<&str> = HashSet::new();
        for (_, t) in self.store.all::<MedLog>() {
            ticks.insert((t.date.as_str(), t.med_id.as_str()));
            tick_days.insert(t.date.as_str());
        }
        let medlist: Vec<(&String, &Med)> = self.store.all::<Med>().collect();
        let nights: HashMap<&str, SleepView> = self.store.all::<Sleep>().filter_map(|(d, s)| Some((d.as_str(), sleep::view(s)?))).collect();
        let used = |d: &str| food.contains_key(d) || mins.contains_key(d) || tick_days.contains(d) || nights.contains_key(d);
        let mut first = start.clone();
        while first <= today && !used(&first) {
            first = dates::add_days(&first, 1);
        }
        let mut days = vec![];
        let mut med_hist: BTreeMap<String, (String, usize, usize)> = BTreeMap::new(); // group -> (name, taken, due)
        let mut d = first;
        while d <= today {
            let a = food.get(d.as_str()).cloned().unwrap_or_default();
            let m = mins.get(d.as_str()).copied().unwrap_or(0.0);
            let due: Vec<(&String, &Med)> = medlist.iter().filter(|(_, x)| meds::is_due(x, &d)).copied().collect();
            let is_used = used(&d);
            let mut taken = 0;
            for (id, med) in &due {
                let t = ticks.contains(&(d.as_str(), id.as_str()));
                taken += t as usize;
                if is_used {
                    let e = med_hist.entry(med.group_id.clone()).or_insert((med.name.clone(), 0, 0));
                    e.0 = med.name.clone();
                    e.1 += t as usize;
                    e.2 += 1;
                }
            }
            let target = training::day_target(s.kcal_target, burns.get(d.as_str()).copied().unwrap_or(0.0), s.add_burn);
            days.push(HistDay {
                kcal: a.kcal.round(),
                prot: round1(a.prot),
                fiber: a.fiber_known.then(|| round1(a.fiber)),
                nut: (!a.nut.is_empty()).then(|| a.nut.iter().map(|(k, v)| (k.clone(), nutrients::round(*v))).collect()),
                mins: m,
                taken,
                due: due.len(),
                used: is_used,
                prot_met: a.prot >= s.prot_floor && a.prot > 0.0,
                fiber_met: a.fiber_known && a.fiber >= s.fiber_floor,
                over: a.kcal > target,
                target: target.round(),
                sleep: nights.get(d.as_str()).cloned(),
                date: d.clone(),
            });
            d = dates::add_days(&d, 1);
        }
        let used_days: Vec<&HistDay> = days.iter().filter(|d| d.used).collect();
        let food_days: Vec<&HistDay> = days.iter().filter(|d| d.kcal > 0.0 || d.prot > 0.0).collect();
        let fd = food_days.len().max(1) as f64;
        let fiber_days: Vec<f64> = food_days.iter().filter_map(|d| d.fiber).collect();
        let due: usize = used_days.iter().map(|d| d.due).sum();
        let taken: usize = used_days.iter().map(|d| d.taken).sum();
        let avg = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
        // Per nutrient: average over the food days that had a value for it.
        let mut nsum = Nut::new();
        let mut nknown: BTreeMap<String, usize> = BTreeMap::new();
        for d in &food_days {
            if let Some(n) = &d.nut {
                for (k, v) in n {
                    *nsum.entry(k.clone()).or_default() += v;
                    *nknown.entry(k.clone()).or_default() += 1;
                }
            }
        }
        let navg: Nut = nsum.iter().map(|(k, v)| (k.clone(), v / nknown[k] as f64)).collect();
        let nutrient_views = nut_views(&navg, &nknown, food_days.len(), s.fiber_floor);
        let slept: Vec<&SleepView> = days.iter().filter_map(|d| d.sleep.as_ref()).collect();
        let clock = |f: fn(&SleepView) -> &str| slept.iter().filter_map(|v| sleep::parse_hhmm(f(v))).collect::<Vec<u32>>();
        let summary = HistSummary {
            avg_kcal: if food_days.is_empty() { 0.0 } else { (food_days.iter().map(|d| d.kcal).sum::<f64>() / fd).round() },
            avg_prot: if food_days.is_empty() { 0.0 } else { round1(food_days.iter().map(|d| d.prot).sum::<f64>() / fd) },
            avg_fiber: (!fiber_days.is_empty()).then(|| round1(avg(&fiber_days))),
            prot_days: used_days.iter().filter(|d| d.prot_met).count(),
            fiber_days: used_days.iter().filter(|d| d.fiber_met).count(),
            food_days: food_days.len(),
            adherence: (due > 0).then(|| (taken as f64 * 100.0 / due as f64).round()),
            days_trained: days.iter().filter(|d| d.mins > 0.0).count(),
            days_used: used_days.len(),
            nutrients: nutrient_views,
            avg_sleep_mins: (!slept.is_empty()).then(|| (slept.iter().map(|v| v.mins).sum::<f64>() / slept.len() as f64).round()),
            // Bedtimes average around midnight (pivot noon); wake times around morning (pivot 18:00).
            avg_bed: sleep::avg_clock(&clock(|v| &v.bed), 12 * 60),
            avg_wake: sleep::avg_clock(&clock(|v| &v.wake), 18 * 60),
            sleep_nights: slept.len(),
        };
        let meds: Vec<MedHist> =
            med_hist.into_values().map(|(name, taken, due)| MedHist { name, taken, due, missed: due - taken }).collect();
        let insights = self.insights(&days, &summary, &meds, &s);
        days.reverse(); // newest first
        History {
            range: range as usize,
            kcal_target: s.kcal_target,
            prot_floor: s.prot_floor,
            fiber_floor: s.fiber_floor,
            days,
            summary,
            meds,
            insights,
        }
    }

    fn insights(&self, days: &[HistDay], sum: &HistSummary, meds: &[MedHist], s: &Settings) -> Vec<Insight> {
        let mut out = vec![];
        let food: Vec<&HistDay> = days.iter().filter(|d| d.kcal > 0.0 || d.prot > 0.0).collect();
        let n = food.len();
        if days.is_empty() {
            return vec![Insight::info(
                "Nothing logged yet",
                "Log a few days of food and this page will show what's working and what's missing.",
            )];
        }
        let gaps = days.iter().filter(|d| d.kcal == 0.0 && d.date != self.today()).count();
        if gaps > 0 {
            out.push(Insight::info(
                &format!("{gaps} day{} without food logged", if gaps == 1 { "" } else { "s" }),
                "Averages only count days with food, so gaps can hide a pattern.",
            ));
        }
        // Sleep: needs a few nights before it says anything.
        if let (Some(avg), true) = (sum.avg_sleep_mins, sum.sleep_nights >= 3) {
            let hours = format_mins(avg);
            let times = match (&sum.avg_bed, &sum.avg_wake) {
                (Some(b), Some(w)) => format!(" Usually in bed by {b}, up at {w}."),
                _ => String::new(),
            };
            if avg < 7.0 * 60.0 {
                out.push(Insight::warn(&format!("Sleeping {hours} a night on average"), &format!("Most adults need 7–9 hours.{times}")));
            } else {
                out.push(Insight::good(&format!("Sleeping {hours} a night"), &format!("Within the 7–9 hours most adults need.{times}")));
            }
        }
        if n == 0 {
            return out;
        }
        let tips = |which: &str| crate::food::richest(which, 4).join(", ");
        let prot_short = food.iter().filter(|d| d.prot < s.prot_floor).count();
        if prot_short == 0 {
            out.push(Insight::good(
                "Protein floor met every day",
                &format!("Averaging {} g against your {} g floor.", sum.avg_prot, s.prot_floor),
            ));
        } else {
            let mut top: Vec<(&str, f64)> = crate::food::TABLE.iter().map(|(n, _, p)| (*n, *p)).collect();
            top.sort_by(|a, b| b.1.total_cmp(&a.1));
            let names: Vec<String> = top.iter().take(4).map(|(n, _)| crate::food::capitalize(n)).collect();
            out.push(Insight {
                level: if prot_short * 3 > n { "warn" } else { "info" }.into(),
                title: format!("Protein under {} g on {prot_short} of {n} days", s.prot_floor),
                detail: format!("Average {} g. Highest per 100 g in your table: {}.", sum.avg_prot, names.join(", ")),
            });
        }
        match sum.avg_fiber {
            Some(f) if f < s.fiber_floor => out.push(Insight::warn(
                &format!("Fiber averaged {f} g, under your {} g floor", s.fiber_floor),
                &format!("Richest in your table: {}.", tips("fiber")),
            )),
            Some(f) => out.push(Insight::good("Fiber on track", &format!("Averaging {f} g against your {} g floor.", s.fiber_floor))),
            None => out.push(Insight::info("No fiber data yet", "Foods logged from now on carry fiber; older entries don't.")),
        }
        let over = food.iter().filter(|d| d.over).count();
        if over * 3 > n {
            out.push(Insight::warn(
                &format!("Over your calorie target on {over} of {n} days"),
                &format!("Average {} kcal against {} kcal.", sum.avg_kcal, s.kcal_target),
            ));
        } else if sum.avg_kcal < s.kcal_target - 400.0 {
            out.push(Insight::info(
                &format!("Averaging {} kcal, well under target", sum.avg_kcal),
                "A big deficit makes it harder to protect protein and muscle.",
            ));
        }
        // Nutrients known on at least half the food days.
        for m in sum.nutrients.iter().filter(|m| m.key != "fiber" && m.coverage >= 0.5) {
            let Some(p) = m.pct else { continue };
            if m.kind == "target" && p < 60.0 {
                let good = crate::food::richest(&m.key, 4);
                out.push(Insight::warn(
                    &format!("{} at {p:.0}% of the daily amount", m.label),
                    &format!(
                        "About {} {} a day against {} {}, from food only (supplements not counted).{}",
                        m.amount,
                        m.unit,
                        m.reference,
                        m.unit,
                        if good.is_empty() { String::new() } else { format!(" Good sources: {}.", good.join(", ")) }
                    ),
                ));
            } else if m.kind == "limit" && p > 100.0 {
                out.push(Insight::warn(
                    &format!("{} above your limit", m.label),
                    &format!("About {} {} a day against a limit of {} {}.", m.amount, m.unit, m.reference, m.unit),
                ));
            }
        }
        let thin: Vec<&str> =
            sum.nutrients.iter().filter(|m| m.kind != "info" && m.coverage > 0.0 && m.coverage < 0.5).map(|m| m.label.as_str()).collect();
        if !thin.is_empty() && thin.len() <= 6 {
            out.push(Insight::info(
                "Some nutrients are only partly known",
                &format!("{} — log foods from the food database to fill these in.", thin.join(", ")),
            ));
        }
        let missed: Vec<&MedHist> = meds.iter().filter(|m| m.missed > 0).collect();
        if meds.iter().any(|m| m.due > 0) && missed.is_empty() {
            out.push(Insight::good("Every medicine dose taken", "Nothing missed in this period."));
        }
        for m in missed {
            out.push(Insight {
                level: if m.missed * 5 > m.due { "warn" } else { "info" }.into(),
                title: format!("{} missed {} of {} times", m.name, m.missed, m.due),
                detail: "Counting only days you used the app.".into(),
            });
        }
        let last = days.iter().rev().position(|d| d.mins > 0.0);
        match last {
            Some(i) if i >= 4 => {
                out.push(Insight::info(&format!("No training in the last {i} days"), "Even a walk counts — log it on the Training tab."))
            }
            None if days.len() >= 7 => {
                out.push(Insight::info("No training logged in this period", "Log sessions on the Training tab to see burn here."))
            }
            _ => {}
        }
        // Warnings first, then info, then good news.
        out.sort_by_key(|i| match i.level.as_str() {
            "warn" => 0,
            "info" => 1,
            _ => 2,
        });
        out
    }

    // ---------- Settings ----------

    pub fn settings(&self) -> Settings {
        self.store.settings()
    }

    pub fn save_settings(&mut self, mut s: Settings) -> Result<()> {
        s.validate()?;
        s.weight_kg = round1(s.weight_kg);
        s.kcal_target = s.kcal_target.round();
        s.prot_floor = s.prot_floor.round();
        s.fiber_floor = s.fiber_floor.round();
        self.store.put("main", s)
    }

    pub fn ai_settings_view(&self) -> AiSettingsView {
        let a = self.store.ai_settings();
        let cfg = self.ai_config();
        AiSettingsView {
            order: cfg.order.iter().map(|p| p.id().to_string()).collect(),
            gemini_key_set: !a.gemini_key.trim().is_empty(),
            gemini_model: cfg.gemini_model.clone(),
            claude_model: a.claude_model,
            claude_path: a.claude_path,
            claude_found: if cfg!(feature = "cli") { ai::find_claude(&cfg).map(|p| p.display().to_string()) } else { None },
        }
    }

    pub fn save_ai_settings(&mut self, i: AiSettingsInput) -> Result<()> {
        let mut a = self.store.ai_settings();
        let order: Vec<String> = i.order.into_iter().filter(|p| p == "claude" || p == "gemini").collect();
        if !order.is_empty() {
            a.order = order;
        }
        let gm = i.gemini_model.trim();
        if !valid_model_name(gm) || !valid_model_name(i.claude_model.trim()) {
            return Err("Model names may only use letters, digits, dots and dashes (up to 40).".into());
        }
        a.gemini_model = gm.to_string();
        a.claude_model = i.claude_model.trim().to_string();
        let path = i.claude_path.trim();
        if !path.is_empty() && !Path::new(path).is_absolute() {
            return Err("The Claude path must be a full path, like /opt/homebrew/bin/claude.".into());
        }
        a.claude_path = path.to_string();
        if i.clear_gemini_key {
            a.gemini_key.clear();
        } else if !i.gemini_key.trim().is_empty() {
            let k = i.gemini_key.trim();
            if k.len() > 200 || k.chars().any(|c| c.is_whitespace() || c.is_control()) {
                return Err("That Gemini key doesn't look right.".into());
            }
            a.gemini_key = k.to_string();
        }
        self.store.put("ai", a)
    }

    pub fn prepare_test_ai(&self) -> (AiConfig, AiRequest) {
        (self.ai_config(), AiRequest { prompt: ai::TEST_PROMPT.into(), image: None })
    }

    // ---------- Rules ----------

    pub fn rules(&self) -> String {
        self.store.rules_text()
    }

    pub fn save_rules(&mut self, text: &str) -> Result<()> {
        let text: String = text.chars().take(4000).collect();
        self.store.put("house", Rules { text })
    }

    pub fn prepare_rules_chat(&self, message: &str) -> Result<(AiConfig, AiRequest)> {
        let m = ai::clip(message, 1000);
        if m.is_empty() {
            return Err("Type a message first.".into());
        }
        Ok((self.ai_config(), AiRequest { prompt: ai::rules_prompt(&self.rules(), &m), image: None }))
    }

    pub fn finish_rules_chat(ans: AiAnswer) -> RulesReply {
        let (reply, change) = ai::clean_rules_reply(&ans.value);
        RulesReply {
            reply: if reply.is_empty() { "I couldn't think of a reply.".into() } else { reply },
            change,
            provider_text: worked_it_out(ans.provider),
        }
    }

    pub fn apply_rule_change(&mut self, change: &RuleChange) -> Result<String> {
        let c = RuleChange { add: ai::clip(&change.add, 200), remove: change.remove.as_deref().map(|r| ai::clip(r, 200)) };
        let text = ai::apply_rule_change(&self.rules(), &c);
        self.save_rules(&text)?;
        Ok(text)
    }

    // ---------- Backup ----------

    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&crate::backup::export(&self.store, self.now())).unwrap_or_default()
    }

    pub fn import_json(&mut self, text: &str) -> Result<usize> {
        crate::backup::import(&mut self.store, text)
    }
}

/// The planned meal at `idx`, checked against the name the screen showed, so a second
/// click (or a stale screen) can't hit the meal that moved into that slot.
fn plan_meal_at<'a>(meals: &'a [PlanMeal], idx: usize, expect: Option<&str>) -> Result<&'a PlanMeal> {
    match meals.get(idx) {
        Some(m) if expect.is_none_or(|n| n == m.name) => Ok(m),
        _ => Err("That meal isn't in the plan any more.".into()),
    }
}

/// 450 → "7 h 30 min"; 420 → "7 h".
pub fn format_mins(m: f64) -> String {
    let m = m.round() as i64;
    match (m / 60, m % 60) {
        (0, r) => format!("{r} min"),
        (h, 0) => format!("{h} h"),
        (h, r) => format!("{h} h {r} min"),
    }
}
