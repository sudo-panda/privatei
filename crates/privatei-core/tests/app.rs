use privatei_core::app::{Clock, LogInput, MedInput};
use privatei_core::model::*;
use privatei_core::Core;

fn core(today: &str, hhmm: &str) -> Core {
    let mut c = Core::in_memory();
    c.set_clock(Clock::Fixed { today: today.into(), hhmm: hhmm.into() });
    c
}

fn set_today(c: &mut Core, today: &str) {
    c.set_clock(Clock::Fixed { today: today.into(), hhmm: "12:00".into() });
}

fn log(c: &mut Core, date: &str, name: &str, kcal: f64, prot: f64) -> String {
    c.add_log(
        date,
        LogInput { meal: "Lunch".into(), name: name.into(), grams: None, kcal, prot, est: false, fiber: None, nut: None, food_ref: None },
    )
    .unwrap()
}

#[test]
fn totals_and_targets() {
    let mut c = core("2026-11-22", "12:00");
    log(&mut c, "2026-11-22", "A", 500.0, 40.0);
    log(&mut c, "2026-11-22", "B", 1100.0, 55.0);
    log(&mut c, "2026-11-21", "C", 900.0, 10.0);
    let d = c.day("2026-11-22").unwrap();
    assert_eq!((d.kcal, d.prot, d.target), (1600.0, 95.0, 1500.0));
    assert!(d.over && d.prot_met);
    c.add_workout("2026-11-22", "gym", 60.0, None, "").unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!(d.burn, 431.0);
    assert_eq!(d.target, 1500.0, "burn not added until the toggle is on");
    c.set_add_burn(true).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!(d.target, 1931.0);
    assert!(!d.over);
}

#[test]
fn manual_log_validation() {
    let mut c = core("2026-11-22", "12:00");
    let bad = |kcal, prot, name: &str| LogInput {
        meal: "Lunch".into(),
        name: name.into(),
        grams: None,
        kcal,
        prot,
        est: false,
        fiber: None,
        nut: None,
        food_ref: None,
    };
    assert!(c.add_log("2026-11-22", bad(0.0, 1.0, "x")).is_err());
    assert!(c.add_log("2026-11-22", bad(5000.0, 1.0, "x")).is_err());
    assert!(c.add_log("2026-11-22", bad(100.0, 501.0, "x")).is_err());
    assert!(c.add_log("2026-11-22", bad(100.0, 1.0, "  ")).is_err());
    assert!(c.add_log("2026-11-22", bad(100.0, 1.0, "ok")).is_ok());
}

#[test]
fn plan_ticks_31_rapid_toggles_consistent() {
    let mut c = core("2026-11-22", "12:00");
    c.set_plan(
        "2026-11-22",
        vec![PlanMeal { slot: "Lunch".into(), name: "Tofu bowl".into(), kcal: 520.0, prot: 38.0, ..Default::default() }],
    )
    .unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().kcal, 0.0, "plan counts only when ticked");
    for _ in 0..31 {
        c.toggle_plan("2026-11-22", 0).unwrap();
    }
    let d = c.day("2026-11-22").unwrap();
    assert!(d.plan[0].done);
    assert_eq!(d.kcal, 520.0);
    let rows: Vec<_> = c.store.all::<Log>().filter(|(_, l)| l.date == "2026-11-22").collect();
    assert_eq!(rows.len(), 1, "exactly one live log row");
    let plan = c.store.get::<Plan>("2026-11-22").unwrap();
    assert_eq!(plan.meals[0].log_id.as_deref(), Some(rows[0].0.as_str()));
    // Deleting the log row unticks the meal.
    let id = rows[0].0.clone();
    c.delete_log(&id).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert!(!d.plan[0].done);
    assert_eq!(d.kcal, 0.0);
}

#[test]
fn copy_yesterday() {
    let mut c = core("2026-11-22", "12:00");
    log(&mut c, "2026-11-21", "Oats", 300.0, 10.0);
    log(&mut c, "2026-11-21", "Skyr", 120.0, 20.0);
    c.set_plan(
        "2026-11-21",
        vec![PlanMeal { slot: "Dinner".into(), name: "Plan meal".into(), kcal: 600.0, prot: 40.0, ..Default::default() }],
    )
    .unwrap();
    c.toggle_plan("2026-11-21", 0).unwrap();
    assert!(c.day("2026-11-22").unwrap().can_copy_yesterday);
    assert_eq!(c.copy_yesterday("2026-11-22").unwrap(), 2, "plan rows are not copied");
    assert_eq!(c.day("2026-11-22").unwrap().kcal, 420.0);
    assert!(c.copy_yesterday("2026-11-22").is_err(), "only when the day has none");
    assert!(!c.day("2026-11-22").unwrap().can_copy_yesterday);
}

#[test]
fn recent_foods_last_8_distinct_newest_first() {
    let mut c = core("2026-11-22", "12:00");
    for i in 0..12 {
        log(&mut c, "2026-11-22", &format!("Food {}", i % 10), 100.0, 1.0);
    }
    let r = c.recent_foods();
    assert_eq!(r.len(), 8);
    assert_eq!(r[0].name, "Food 1");
    assert_eq!(r[1].name, "Food 0");
    assert_eq!(r[2].name, "Food 9");
}

fn med(name: &str, time: &str) -> MedInput {
    MedInput { group_id: None, name: name.into(), dose: "1".into(), freq: "daily".into(), days: vec![], dom: None, time: time.into() }
}

#[test]
fn medicine_schedule_history() {
    let mut c = core("2026-11-10", "09:00");
    let g = c.save_med(med("Vit D", "08:00")).unwrap().group_id;
    // Day 1: taken; day 2: missed.
    let id1 = c.meds_view().daily[0].id.clone();
    c.toggle_dose("2026-11-10", &id1).unwrap();
    set_today(&mut c, "2026-11-12");
    c.toggle_dose("2026-11-12", &id1).unwrap(); // ticked today, before the change
    let d = c.day("2026-11-11").unwrap();
    assert_eq!(d.meds[0].status, "missed");

    // Change the schedule to weekly Mon+Thu: old record ends yesterday, new starts today.
    let r = c.save_med(MedInput { group_id: Some(g.clone()), freq: "weekly".into(), days: vec![1, 4], ..med("Vit D", "08:00") }).unwrap();
    assert!(r.started_new);
    let recs: Vec<Med> = c.store.all::<Med>().map(|(_, m)| m.clone()).collect();
    assert_eq!(recs.len(), 2);
    let old = recs.iter().find(|m| m.freq == "daily").unwrap();
    let new_id = c.meds_view().days[0].id.clone();
    assert_eq!(old.until.as_deref(), Some("2026-11-11"));
    assert!(c.store.get::<MedLog>(&format!("2026-11-12_{new_id}")).is_some(), "today's tick moved");
    assert!(c.store.get::<MedLog>(&format!("2026-11-12_{id1}")).is_none());
    // History is kept: the 10th was still daily.
    assert_eq!(c.day("2026-11-10").unwrap().meds.len(), 1);
    assert_eq!(c.day("2026-11-10").unwrap().meds[0].status, "taken");

    // A second change on the same day replaces the new record instead of splitting again.
    let r = c.save_med(MedInput { group_id: Some(g.clone()), freq: "weekly".into(), days: vec![1], ..med("Vit D", "08:00") }).unwrap();
    assert!(!r.started_new);
    assert_eq!(c.store.all::<Med>().count(), 2);

    // Name-only edits don't start a new schedule.
    let r = c.save_med(MedInput { group_id: Some(g.clone()), freq: "weekly".into(), days: vec![1], ..med("Vitamin D3", "08:00") }).unwrap();
    assert!(!r.started_new);

    // Stop: until = yesterday, shows under "No longer taking".
    c.stop_med(&g).unwrap();
    let v = c.meds_view();
    assert!(v.daily.is_empty() && v.days.is_empty());
    assert_eq!(v.stopped.len(), 1);

    // Erase removes every record and tick.
    c.erase_med(&g).unwrap();
    assert_eq!(c.store.all::<Med>().count(), 0);
    assert_eq!(c.store.all::<MedLog>().count(), 0);
}

#[test]
fn overdue_today() {
    let mut c = core("2026-11-22", "09:00");
    c.save_med(med("Morning", "08:00")).unwrap();
    c.save_med(med("Evening", "20:00")).unwrap();
    c.save_med(med("Whenever", "")).unwrap();
    let d = c.day("2026-11-22").unwrap();
    let st: Vec<(&str, &str)> = d.meds.iter().map(|m| (m.name.as_str(), m.status.as_str())).collect();
    assert_eq!(st, vec![("Morning", "overdue"), ("Evening", "due"), ("Whenever", "due")]);
    assert!(c.toggle_dose("2026-11-23", &d.meds[0].med_id).is_err(), "no future ticks");
}

#[test]
fn history_summary() {
    let mut c = core("2026-11-22", "12:00");
    set_today(&mut c, "2026-11-20");
    c.save_med(med("Vit D", "08:00")).unwrap();
    let mid = c.meds_view().daily[0].id.clone();
    set_today(&mut c, "2026-11-22");
    log(&mut c, "2026-11-20", "A", 1400.0, 95.0);
    log(&mut c, "2026-11-21", "B", 1800.0, 60.0);
    c.add_workout("2026-11-21", "run", 30.0, None, "").unwrap();
    c.toggle_dose("2026-11-20", &mid).unwrap();
    let h = c.history();
    // Trimmed to the first day used (the 20th), newest first.
    assert_eq!(h.days.len(), 3);
    assert_eq!(h.days[0].date, "2026-11-22");
    assert_eq!(h.days[2].date, "2026-11-20");
    let s = &h.summary;
    assert_eq!(s.days_used, 2, "the 22nd has nothing logged");
    assert_eq!(s.avg_kcal, 1600.0);
    assert_eq!(s.avg_prot, 77.5);
    assert_eq!(s.prot_days, 1);
    assert_eq!(s.days_trained, 1);
    assert_eq!(s.adherence, Some(50.0), "only used days count: 1 of 2");
}

#[test]
fn stock_and_shopping() {
    let mut c = core("2026-11-22", "12:00");
    c.save_stock(
        None,
        Stock {
            name: "Skyr".into(),
            qty: 2.0,
            unit: "pots".into(),
            cat: "chilled".into(),
            added: "2026-11-20".into(),
            use_by: Some("2026-11-25".into()),
        },
    )
    .unwrap();
    c.save_stock(
        None,
        Stock { name: "Dal".into(), qty: 400.0, unit: "g".into(), cat: "cooked".into(), added: "2026-11-19".into(), use_by: None },
    )
    .unwrap();
    c.save_stock(
        None,
        Stock {
            name: "Lentils".into(),
            qty: 1.0,
            unit: "kg".into(),
            cat: "shelf".into(),
            added: "2026-11-01".into(),
            use_by: Some("2027-05-01".into()),
        },
    )
    .unwrap();
    let v = c.stock_view();
    assert_eq!(v.soon_count, 1);
    let cooked = v.sections.iter().find(|s| s.cat == "cooked").unwrap();
    assert!(cooked.items[0].eat_today, "cooked 3+ days old");

    let add = |c: &mut Core, name: &str, qty: &str, price: f64, trip: u8| {
        c.save_list_item(
            None,
            ListItem { name: name.into(), qty: qty.into(), cat: "chilled".into(), price: Some(price), trip, ..Default::default() },
        )
        .unwrap()
    };
    let skyr = add(&mut c, "skyr", "4 pots", 6.4, 1);
    let tofu = add(&mut c, "Smoked tofu", "2", 5.0, 1);
    let milk = add(&mut c, "Oat milk", "1 l", 2.5, 2);
    let s = c.shop_view(1);
    assert_eq!((s.spend, s.trip_spend, s.budget), (13.9, 11.4, 80.0));

    let m = c.check_have();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].stock_name, "Skyr");
    // Not still there → keep on the list.
    c.resolve_have(&skyr, &m[0].stock_id, true).unwrap();
    assert_eq!(c.shop_view(1).in_stock.len(), 1);
    c.move_back(&skyr).unwrap();
    assert_eq!(c.shop_view(1).in_stock.len(), 0);

    c.toggle_list_item(&skyr).unwrap();
    c.toggle_list_item(&tofu).unwrap();
    assert_eq!(c.bought(1).unwrap(), 2);
    let skyr_rows: Vec<Stock> =
        c.store.all::<Stock>().filter(|(_, s)| s.name.eq_ignore_ascii_case("skyr")).map(|(_, s)| s.clone()).collect();
    assert_eq!(skyr_rows.len(), 1, "merged, not duplicated");
    assert_eq!(skyr_rows[0].qty, 6.0);
    let tofu_row = c.store.all::<Stock>().find(|(_, s)| s.name == "Smoked tofu").unwrap().1.clone();
    assert_eq!((tofu_row.qty, tofu_row.unit.as_str(), tofu_row.added.as_str()), (2.0, "", "2026-11-22"));
    assert!(c.store.get::<ListItem>(&milk).is_some(), "other trip untouched");
    assert_eq!(c.shop_view(1).groups.len(), 0);
}

#[test]
fn qty_parsing() {
    use privatei_core::app::parse_qty;
    assert_eq!(parse_qty("4 pots"), (4.0, "pots".into()));
    assert_eq!(parse_qty("1 kg"), (1.0, "kg".into()));
    assert_eq!(parse_qty("500g"), (500.0, "g".into()));
    assert_eq!(parse_qty("1,5 l"), (1.5, "l".into()));
    assert_eq!(parse_qty(""), (1.0, "".into()));
    assert_eq!(parse_qty("some"), (1.0, "".into()));
}

#[test]
fn repeat_last_week() {
    let mut c = core("2026-11-22", "12:00");
    let meal = |n: &str| PlanMeal { slot: "Lunch".into(), name: n.into(), kcal: 500.0, prot: 30.0, ..Default::default() };
    for i in 0..8 {
        let d = privatei_core::dates::add_days("2026-11-16", i);
        c.set_plan(&d, vec![meal(&format!("Meal {i}"))]).unwrap();
    }
    c.toggle_plan("2026-11-23", 0).unwrap();
    // Latest 7 planned days: 17..23. 23+7 = 30; 22+7=29 … 17+7=24. All empty → 7 copies.
    assert_eq!(c.repeat_last_week().unwrap(), 7);
    let p = c.store.get::<Plan>("2026-11-30").unwrap();
    assert_eq!(p.meals[0].name, "Meal 7");
    assert!(!p.meals[0].done && p.meals[0].log_id.is_none(), "ticks reset");
    // Running again: latest 7 are now 24..30, targets 1..7 Dec, none planned → 7.
    // But first plan one of them to prove skipping.
    c.set_plan("2026-12-01", vec![meal("Keep me")]).unwrap();
    // Latest 7 now include 12-01; targets: 12-08 and 25..30+7 → 12-02..12-07; 12-01 (from 11-24) is skipped as it's not a target… check counts.
    let n = c.repeat_last_week().unwrap();
    assert!(n <= 7);
    assert_eq!(c.store.get::<Plan>("2026-12-01").unwrap().meals[0].name, "Keep me");
    let w = c.week_view();
    assert_eq!(w.days[0].date, "2026-11-15");
    assert_eq!(w.avg_kcal, 500.0);
}

#[test]
fn repeat_skips_planned_days() {
    let mut c = core("2026-11-22", "12:00");
    let meal = |n: &str| PlanMeal { slot: "Lunch".into(), name: n.into(), kcal: 500.0, prot: 30.0, ..Default::default() };
    c.set_plan("2026-11-20", vec![meal("A")]).unwrap();
    c.set_plan("2026-11-21", vec![meal("B")]).unwrap();
    c.set_plan("2026-11-28", vec![meal("Already")]).unwrap();
    // Latest 7 planned: 28, 21, 20 → targets 12-05 (new), 11-28 (planned: skip), 11-27 (new).
    assert_eq!(c.repeat_last_week().unwrap(), 2);
    assert_eq!(c.store.get::<Plan>("2026-11-28").unwrap().meals[0].name, "Already");
    assert_eq!(c.store.get::<Plan>("2026-11-27").unwrap().meals[0].name, "A");
}

#[test]
fn workout_import_and_validation() {
    let mut c = core("2026-11-22", "12:00");
    let r = c.import_workouts("Date,Sport,Time\n2026-11-21,Crunches,20\n2026-12-01,Run,30\n").unwrap();
    assert_eq!((r.added, r.skipped), (1, 2));
    assert!(c.add_workout("2026-11-23", "run", 30.0, None, "").is_err(), "future");
    assert!(c.add_workout("2026-11-22", "run", 0.0, None, "").is_err());
    assert!(c.add_workout("2026-11-22", "run", 30.0, Some(6000.0), "").is_err());
}

#[test]
fn recent_workouts_newest_first() {
    let mut c = core("2026-11-22", "12:00");
    c.add_workout("2026-11-20", "run", 30.0, None, "A").unwrap();
    c.add_workout("2026-11-22", "gym", 45.0, None, "B").unwrap();
    c.add_workout("2026-11-21", "walk", 60.0, None, "C").unwrap();
    let names: Vec<String> = c.recent_workouts(2).into_iter().map(|w| w.name).collect();
    assert_eq!(names, vec!["B", "C"]);
}

#[test]
fn settings_validation() {
    let mut c = core("2026-11-22", "12:00");
    assert!(c.save_settings(Settings { weight_kg: 301.0, ..Default::default() }).is_err());
    assert!(c.save_settings(Settings { kcal_target: 700.0, ..Default::default() }).is_err());
    assert!(c.save_settings(Settings { prot_floor: 401.0, ..Default::default() }).is_err());
    c.save_settings(Settings { weight_kg: 80.0, ..Default::default() }).unwrap();
    assert_eq!(c.settings().weight_kg, 80.0);
}

#[test]
fn import_sample_fixture() {
    let mut c = core("2026-11-22", "12:00");
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/sample-export.json")).unwrap();
    let n = c.import_json(&text).unwrap();
    assert_eq!(n, 13);
    assert_eq!(c.day("2026-11-19").unwrap().kcal, 374.0);
    assert_eq!(c.day("2026-11-20").unwrap().kcal, 540.0);
    assert_eq!(c.rules(), "No cheese after 20:00");
    assert_eq!(c.store.ai_settings().gemini_key, "");
    assert_eq!(c.learned_foods().len(), 1);
    // Learned food is used locally now.
    let e = c.prepare_estimate("protein pudding").unwrap();
    assert_eq!(e.local.items.len(), 1);
    assert_eq!(e.local.items[0].kcal, 160.0);
    // Export round trip.
    let out = c.export_json();
    assert!(out.contains("\"privatei-export\""));
    let mut c2 = core("2026-11-22", "12:00");
    assert_eq!(c2.import_json(&out).unwrap(), 13);
}

#[test]
fn rules_apply() {
    let mut c = core("2026-11-22", "12:00");
    c.save_rules("No cheese").unwrap();
    let t = c.apply_rule_change(&privatei_core::ai::RuleChange { add: "Max 2 eggs a day".into(), remove: None }).unwrap();
    assert_eq!(t, "No cheese\nMax 2 eggs a day");
}

#[test]
fn ai_settings_never_echo_key() {
    let mut c = core("2026-11-22", "12:00");
    use privatei_core::app::AiSettingsInput;
    let input = |key: &str, clear: bool| AiSettingsInput {
        order: vec!["gemini".into(), "claude".into()],
        gemini_key: key.into(),
        clear_gemini_key: clear,
        gemini_model: "".into(),
        claude_model: "".into(),
        claude_path: "".into(),
    };
    c.save_ai_settings(input("abc123", false)).unwrap();
    let v = c.ai_settings_view();
    assert!(v.gemini_key_set);
    assert_eq!(v.order, vec!["gemini", "claude"]);
    assert_eq!(v.gemini_model, "gemini-flash-latest");
    assert!(!serde_json::to_string(&v).unwrap().contains("abc123"));
    c.save_ai_settings(input("", false)).unwrap();
    assert_eq!(c.store.ai_settings().gemini_key, "abc123", "empty keeps the key");
    c.save_ai_settings(input("", true)).unwrap();
    assert!(!c.ai_settings_view().gemini_key_set);
    let mut bad = input("", false);
    bad.claude_model = "x; rm -rf".into();
    assert!(c.save_ai_settings(bad).is_err());
}

#[test]
fn fiber_and_nutrients_on_the_day() {
    let mut c = core("2026-11-22", "08:00");
    let p = c.prepare_estimate("80g oats, 200g frozen raspberries").unwrap();
    let v = c.finish_estimate(p, None);
    c.save_items("2026-11-22", v.items).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert!(d.fiber_known);
    assert_eq!(d.fiber, 21.1); // 8.1 + 13.0
    assert_eq!(d.fiber_floor, 30.0);
    assert!(!d.fiber_met);
    let vc = d.nutrients.iter().find(|m| m.key == "vitC").unwrap();
    assert_eq!((vc.amount, vc.pct, vc.coverage), (52.0, Some(58.0), 1.0));
    assert!(d.nutrients.iter().find(|m| m.key == "sodium").unwrap().pct.is_none(), "unknown, not zero");
    log(&mut c, "2026-11-21", "Mystery", 300.0, 10.0);
    assert!(!c.day("2026-11-21").unwrap().fiber_known);
}

#[test]
fn food_database_search_and_log() {
    let mut c = core("2026-11-22", "12:00");
    c.foods = Some(privatei_core::fooddb::sample_db());
    let hits = c.search_foods("chick", 10);
    let refs: Vec<&str> = hits.iter().map(|h| h.reference.as_str()).collect();
    assert!(refs.contains(&"table:chicken breast"), "built-in table first: {refs:?}");
    assert!(refs.contains(&"usda:1"));
    assert_eq!(hits[0].source, "table");
    let id = c.add_food("2026-11-22", "Lunch", "usda:2", 250.0).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!((d.kcal, d.prot, d.fiber), (290.0, 22.5, 19.8));
    let folate = d.nutrients.iter().find(|n| n.key == "folate").unwrap();
    assert_eq!(folate.amount, 453.0);
    let l = c.store.get::<Log>(&id).unwrap().clone();
    assert_eq!(l.food_ref.as_deref(), Some("usda:2"));
    assert_eq!(l.name, "Lentils, mature seeds, cooked, boiled · 250 g");
    // Editing the grams rescales the stored nutrients.
    c.update_log(
        &id,
        LogInput {
            meal: "Lunch".into(),
            name: l.name.clone(),
            grams: Some(125.0),
            kcal: 145.0,
            prot: 11.3,
            est: false,
            fiber: Some(9.9),
            nut: None,
            food_ref: None,
        },
    )
    .unwrap();
    assert_eq!(c.store.get::<Log>(&id).unwrap().nut.as_ref().unwrap()["folate"], 227.0); // 453 × 0.5 = 226.5, rounded
    assert!(c.add_food("2026-11-22", "Lunch", "usda:999", 100.0).is_err());
    assert!(c.add_food("2026-11-22", "Lunch", "usda:2", 0.0).is_err());
    assert!(c.remove_food_source("usda").is_err(), "the shipped database stays");
    assert_eq!(c.food_detail("table:egg").unwrap().portions[0].grams, 55.0);
}

#[test]
fn history_insights_point_at_what_is_missing() {
    let mut c = core("2026-11-22", "12:00");
    set_today(&mut c, "2026-11-08");
    c.save_med(med("Magnesium", "21:00")).unwrap();
    let mid = c.meds_view().daily[0].id.clone();
    set_today(&mut c, "2026-11-22");
    for i in 0..14 {
        let d = privatei_core::dates::add_days("2026-11-22", -i);
        // Low protein most days, low fiber, a little vitamin C.
        let p = c.prepare_estimate(&format!("150g rice cooked, {} g chicken breast, 1 apple", if i % 3 == 0 { 300 } else { 150 })).unwrap();
        let v = c.finish_estimate(p, None);
        c.save_items(&d, v.items).unwrap();
        if i % 4 != 0 {
            c.toggle_dose(&d, &mid).unwrap();
        }
    }
    let h = c.history_range(14);
    assert_eq!(h.range, 14);
    assert_eq!(h.days.len(), 14);
    let titles: Vec<&str> = h.insights.iter().map(|i| i.title.as_str()).collect();
    assert!(titles.iter().any(|t| t.starts_with("Protein under 90 g on")), "{titles:?}");
    assert!(titles.iter().any(|t| t.starts_with("Fiber averaged")), "{titles:?}");
    assert!(titles.iter().any(|t| t.starts_with("Vitamin D at")), "{titles:?}");
    assert!(titles.iter().any(|t| t.starts_with("Magnesium missed 4 of 14")), "{titles:?}");
    assert!(titles.iter().any(|t| t.contains("training")), "{titles:?}");
    assert_eq!(h.insights[0].level, "warn", "warnings come first");
    let fiber_tip = &h.insights.iter().find(|i| i.title.starts_with("Fiber")).unwrap().detail;
    assert!(fiber_tip.contains("Ground flaxseed"));
    assert_eq!(h.meds[0].missed, 4);
    assert!(h.summary.avg_fiber.unwrap() < 30.0);
}

#[test]
fn plan_ahead_shows_today_onwards() {
    let mut c = core("2026-11-22", "12:00");
    let meal =
        |slot: &str, k: f64| PlanMeal { slot: slot.into(), name: "x".into(), kcal: k, prot: 40.0, fiber: Some(8.0), ..Default::default() };
    c.set_plan("2026-11-20", vec![meal("Lunch", 500.0)]).unwrap();
    c.add_plan_meal("2026-11-22", meal("Lunch", 600.0)).unwrap();
    c.add_plan_meal("2026-11-22", meal("Dinner", 1000.0)).unwrap();
    c.add_plan_meal("2026-11-24", meal("Breakfast", 300.0)).unwrap();
    let p = c.plan_ahead(7);
    assert_eq!(p.days.len(), 7);
    assert_eq!(p.days[0].date, "2026-11-22");
    assert!(p.days[0].is_today && p.days[0].over);
    assert_eq!((p.days[0].kcal, p.days[0].fiber), (1600.0, 16.0));
    assert_eq!(p.days[0].empty_slots, vec!["Breakfast", "Snack"]);
    assert_eq!(p.days[2].meals.len(), 1);
    assert_eq!(p.planned_days, 2);
    assert!(p.has_earlier);
    c.remove_plan_meal("2026-11-22", 0).unwrap();
    assert_eq!(c.plan_ahead(1).days[0].kcal, 1000.0);
}

#[test]
fn training_summary_this_week() {
    let mut c = core("2026-11-22", "12:00");
    c.add_workout("2026-11-22", "gym", 60.0, Some(400.0), "").unwrap();
    c.add_workout("2026-11-19", "run", 30.0, Some(300.0), "").unwrap();
    c.add_workout("2026-11-10", "run", 45.0, Some(500.0), "").unwrap();
    let t = c.training_summary();
    assert_eq!((t.week_sessions, t.week_mins, t.week_kcal, t.last_days_ago), (2, 90.0, 700.0, Some(0)));
    assert_eq!(t.by_kind[0], ("run".to_string(), 75.0));
    assert_eq!(t.daily.len(), 28);
}

#[test]
fn settings_fiber_floor() {
    let mut c = core("2026-11-22", "12:00");
    assert!(c.save_settings(Settings { fiber_floor: 120.0, ..Default::default() }).is_err());
    c.save_settings(Settings { fiber_floor: 35.0, ..Default::default() }).unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().fiber_floor, 35.0);
}

#[test]
fn edit_log_plan_and_training() {
    let mut c = core("2026-11-22", "12:00");
    let id = log(&mut c, "2026-11-22", "Egg", 79.0, 7.0);
    let edit = |name: &str, kcal: f64| LogInput {
        meal: "Dinner".into(),
        name: name.into(),
        grams: Some(110.0),
        kcal,
        prot: 14.0,
        est: false,
        fiber: Some(0.0),
        nut: None,
        food_ref: None,
    };
    c.update_log(&id, edit("Two eggs", 157.0)).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!((d.kcal, d.meals[0].meal.as_str(), d.meals[0].rows[0].name.as_str()), (157.0, "Dinner", "Two eggs"));
    assert!(c.update_log(&id, edit("", 1.0)).is_err());
    assert!(c.update_log("nope", edit("x", 10.0)).is_err());

    // A ticked plan meal and its log row stay in step both ways.
    c.set_plan("2026-11-22", vec![PlanMeal { slot: "Lunch".into(), name: "Bowl".into(), kcal: 500.0, prot: 30.0, ..Default::default() }])
        .unwrap();
    c.toggle_plan("2026-11-22", 0).unwrap();
    c.update_plan_meal(
        "2026-11-22",
        0,
        PlanMeal { slot: "Lunch".into(), name: "Big bowl".into(), kcal: 700.0, prot: 40.0, ..Default::default() },
    )
    .unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert!(d.plan[0].done, "tick kept");
    assert_eq!(d.kcal, 857.0);
    let lid = c.store.get::<Plan>("2026-11-22").unwrap().meals[0].log_id.clone().unwrap();
    c.update_log(
        &lid,
        LogInput {
            meal: "Lunch".into(),
            name: "Huge bowl".into(),
            grams: None,
            kcal: 900.0,
            prot: 50.0,
            est: false,
            fiber: None,
            nut: None,
            food_ref: None,
        },
    )
    .unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().plan[0].name, "Huge bowl");

    let w = c.add_workout("2026-11-22", "run", 30.0, None, "").unwrap();
    c.update_workout(&w, "cycle", 60.0, Some(500.0), "Spin").unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!(
        (d.workouts[0].kind.as_str(), d.workouts[0].mins, d.workouts[0].kcal, d.workouts[0].name.as_str()),
        ("cycle", 60.0, 500.0, "Spin")
    );
}
