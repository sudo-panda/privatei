use privatei_core::app::{Clock, LogInput, MedInput, ShopDay};
use privatei_core::model::Ingredient;
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
        c.toggle_plan("2026-11-22", 0, None).unwrap();
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
    c.toggle_plan("2026-11-21", 0, None).unwrap();
    assert!(c.day("2026-11-22").unwrap().can_copy_yesterday);
    assert_eq!(c.copy_yesterday("2026-11-22").unwrap(), 2, "plan rows are not copied");
    assert_eq!(c.day("2026-11-22").unwrap().kcal, 420.0);
    assert!(c.copy_yesterday("2026-11-22").is_err(), "only when the day has none");
    assert!(!c.day("2026-11-22").unwrap().can_copy_yesterday);
}

#[test]
fn usual_foods_rank_by_frequency_and_recency() {
    let mut c = core("2026-11-22", "12:00");
    // Skyr every day for three weeks, at two different amounts.
    for i in 0..21 {
        let d = privatei_core::dates::add_days("2026-11-22", -i);
        log(&mut c, &d, if i % 2 == 0 { "Skyr · 200 g" } else { "Skyr · 150 g" }, 126.0, 22.0);
    }
    // Oats twice, long ago; a curry once, today.
    log(&mut c, "2026-10-01", "Oats · 50 g", 185.0, 6.5);
    log(&mut c, "2026-10-02", "Oats · 50 g", 185.0, 6.5);
    log(&mut c, "2026-11-22", "Curry", 600.0, 25.0);
    for i in 0..10 {
        log(&mut c, "2026-08-01", &format!("Old food {i}"), 100.0, 1.0); // older than 90 days: ignored
    }
    let r: Vec<String> = c.recent_foods().into_iter().map(|f| f.name).collect();
    assert_eq!(r[0], "Skyr · 200 g", "most frequent first, as its latest amount: {r:?}");
    assert_eq!(r.iter().filter(|n| n.starts_with("Skyr")).count(), 1, "one chip per food");
    assert_eq!(r[1], "Curry", "something new today still shows up");
    assert_eq!(r[2], "Oats · 50 g");
    assert!(!r.iter().any(|n| n.starts_with("Old food")));
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
    c.toggle_plan("2026-11-23", 0, None).unwrap();
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
    c.save_items("2026-11-22", v.items.into_iter().map(|r| r.item).collect()).unwrap();
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
        c.save_items(&d, v.items.into_iter().map(|r| r.item).collect()).unwrap();
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
    c.remove_plan_meal("2026-11-22", 0, None).unwrap();
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
    assert_eq!((d.kcal, d.meals[3].meal.as_str(), d.meals[3].rows[0].name.as_str()), (157.0, "Dinner", "Two eggs"));
    assert!(c.update_log(&id, edit("", 1.0)).is_err());
    assert!(c.update_log("nope", edit("x", 10.0)).is_err());

    // A ticked plan meal and its log row stay in step both ways.
    c.set_plan("2026-11-22", vec![PlanMeal { slot: "Lunch".into(), name: "Bowl".into(), kcal: 500.0, prot: 30.0, ..Default::default() }])
        .unwrap();
    c.toggle_plan("2026-11-22", 0, None).unwrap();
    c.update_plan_meal(
        "2026-11-22",
        0,
        Some("Bowl"),
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

#[test]
fn recipes_from_ingredients() {
    let mut c = core("2026-11-22", "19:00");
    c.foods = Some(privatei_core::fooddb::sample_db());
    let lentils = c.ingredient("usda:2", 400.0).unwrap();
    assert_eq!((lentils.kcal, lentils.fiber), (464.0, Some(31.6)));
    let oil = c.ingredient("table:oil", 15.0).unwrap();
    let input = |name: &str, ings: Vec<Ingredient>| privatei_core::app::RecipeInput {
        name: name.into(),
        servings: 4.0,
        cooked_grams: Some(1200.0),
        ingredients: ings,
        note: String::new(),
    };
    let preview = c.recipe_preview(input("Dal", vec![lentils.clone(), oil.clone()])).unwrap();
    assert_eq!((preview.total.kcal, preview.per_serving.kcal, preview.per_serving.grams), (597.0, 149.0, 300.0));
    assert!(c.save_recipe(None, input("", vec![oil.clone()])).is_err());
    assert!(c.save_recipe(None, input("Empty", vec![])).is_err());
    let id = c.save_recipe(None, input("Dal", vec![lentils, oil])).unwrap();
    // Recipes come up first in search and log like any food.
    let hits = c.search_foods("dal", 10);
    assert_eq!(hits[0].reference, format!("recipe:{id}"));
    assert_eq!(hits[0].portions[0].grams, 300.0);
    c.add_food("2026-11-22", "Dinner", &format!("recipe:{id}"), 300.0).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!((d.kcal, d.fiber), (149.0, 7.9));
    assert_eq!(d.nutrients.iter().find(|n| n.key == "folate").unwrap().amount, 181.0);
    // It survives an export/import round trip.
    let text = c.export_json();
    let mut c2 = core("2026-11-22", "12:00");
    c2.import_json(&text).unwrap();
    assert_eq!(c2.recipes()[0].name, "Dal");
    c.delete_recipe(&id).unwrap();
    assert!(c.recipes().is_empty());
}

#[test]
fn day_always_has_all_four_meals() {
    let mut c = core("2026-11-22", "12:00");
    let names = |c: &Core| c.day("2026-11-22").unwrap().meals.iter().map(|g| (g.meal.clone(), g.rows.len())).collect::<Vec<_>>();
    let empty = names(&c);
    assert_eq!(empty, vec![("Breakfast".into(), 0), ("Lunch".into(), 0), ("Snack".into(), 0), ("Dinner".into(), 0)]);
    c.add_log(
        "2026-11-22",
        LogInput {
            meal: "Snack".into(),
            name: "Apple".into(),
            grams: None,
            kcal: 80.0,
            prot: 0.3,
            est: false,
            fiber: None,
            nut: None,
            food_ref: None,
        },
    )
    .unwrap();
    assert_eq!(names(&c)[2], ("Snack".into(), 1));
    assert_eq!(names(&c).len(), 4);
}

#[test]
fn plan_meal_actions_check_the_name_so_a_second_click_cant_hit_the_next_meal() {
    let mut c = core("2026-11-22", "12:00");
    let m = |name: &str| PlanMeal { slot: "Lunch".into(), name: name.into(), kcal: 400.0, prot: 20.0, ..Default::default() };
    c.set_plan("2026-11-22", vec![m("Bowl"), m("Soup")]).unwrap();
    c.remove_plan_meal("2026-11-22", 0, Some("Bowl")).unwrap();
    // The double click: index 0 is now Soup.
    assert!(c.remove_plan_meal("2026-11-22", 0, Some("Bowl")).is_err());
    assert!(c.toggle_plan("2026-11-22", 0, Some("Bowl")).is_err());
    assert!(c.update_plan_meal("2026-11-22", 0, Some("Bowl"), m("Stew")).is_err());
    let d = c.day("2026-11-22").unwrap();
    assert_eq!((d.plan.len(), d.plan[0].name.as_str(), d.plan[0].done), (1, "Soup", false));
    assert!(c.toggle_plan("2026-11-22", 0, Some("Soup")).unwrap());
}

#[test]
fn edit_log_renames_the_amount_and_allows_zero_kcal() {
    let mut c = core("2026-11-22", "12:00");
    let edit = |name: &str, g: f64, kcal: f64| LogInput {
        meal: "Lunch".into(),
        name: name.into(),
        grams: Some(g),
        kcal,
        prot: 0.0,
        est: false,
        fiber: None,
        nut: None,
        food_ref: None,
    };
    let id = c.add_log("2026-11-22", edit("Skyr · 200 g", 200.0, 124.0)).unwrap();
    c.update_log(&id, edit("Skyr · 200 g", 100.0, 62.0)).unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().meals[1].rows[0].name, "Skyr · 100 g");
    // 0 kcal (water, black coffee) can be edited like anything else.
    c.update_log(&id, edit("Sparkling water · 100 g", 250.0, 0.0)).unwrap();
    let row = &c.day("2026-11-22").unwrap().meals[1].rows[0];
    assert_eq!((row.name.as_str(), row.kcal), ("Sparkling water · 250 g", 0.0));
    // A name without the old amount is left as typed.
    c.update_log(&id, edit("Water", 300.0, 0.0)).unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().meals[1].rows[0].name, "Water");
    assert!(c.update_log(&id, edit("x", 10.0, -1.0)).is_err());
}

#[test]
fn meal_fiber_subtotal_counts_known_rows_and_stays_unknown_without_any() {
    let mut c = core("2026-11-22", "12:00");
    let row = |name: &str, fiber: Option<f64>| LogInput {
        meal: "Lunch".into(),
        name: name.into(),
        grams: None,
        kcal: 100.0,
        prot: 1.0,
        est: false,
        fiber,
        nut: None,
        food_ref: None,
    };
    c.add_log("2026-11-22", row("Lentils", Some(7.94))).unwrap();
    c.add_log("2026-11-22", row("Rice", Some(0.6))).unwrap();
    c.add_log("2026-11-22", row("Mystery", None)).unwrap();
    let d = c.day("2026-11-22").unwrap();
    assert_eq!(d.meals[1].fiber, Some(8.5));
    assert_eq!(d.meals[0].fiber, None, "an empty meal's fiber is unknown, not 0");
}

#[test]
fn sleep_in_the_day_and_in_history() {
    let mut c = core("2026-11-22", "12:00");
    c.set_sleep("2026-11-22", "23:30", "7:00").unwrap();
    let s = c.day("2026-11-22").unwrap().sleep.unwrap();
    assert_eq!((s.bed.as_str(), s.wake.as_str(), s.mins), ("23:30", "07:00", 450.0));
    assert!(c.day("2026-11-21").unwrap().sleep.is_none());
    // Saving again replaces the night; clearing removes it.
    c.set_sleep("2026-11-22", "00:30", "07:00").unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().sleep.unwrap().mins, 390.0);
    assert!(c.set_sleep("2026-11-23", "23:00", "07:00").is_err(), "future");
    assert!(c.set_sleep("2026-11-21", "25:00", "07:00").is_err());
    assert!(c.set_sleep("2026-11-21", "07:00", "07:00").is_err());
    c.set_sleep("2026-11-21", "22:30", "06:30").unwrap();
    c.set_sleep("2026-11-20", "23:30", "06:00").unwrap();
    let h = c.history_range(14);
    assert_eq!(h.summary.sleep_nights, 3);
    assert_eq!(h.summary.avg_sleep_mins, Some(((390 + 480 + 390) / 3) as f64));
    assert_eq!(h.summary.avg_bed.as_deref(), Some("23:30"), "bedtimes average across midnight");
    assert_eq!(h.summary.avg_wake.as_deref(), Some("06:30"));
    assert_eq!(h.days[0].sleep.as_ref().unwrap().bed, "00:30");
    assert!(h.insights.iter().any(|i| i.title.starts_with("Sleeping 7 h a night")), "{:?}", h.insights);
    // Days with only sleep count as used, so History doesn't trim them away.
    assert_eq!(h.days.last().unwrap().date, "2026-11-20");
    c.clear_sleep("2026-11-22").unwrap();
    assert!(c.day("2026-11-22").unwrap().sleep.is_none());
    // Backups carry sleep.
    let dump = privatei_core::backup::export(&c.store, 0);
    assert_eq!(dump["collections"]["sleep"]["2026-11-21"]["bed"], "22:30");
}

#[test]
fn shopping_days_on_the_plan_and_the_log() {
    let mut c = core("2026-11-22", "12:00");
    let item = |name: &str, trip: u8, price: Option<f64>, checked: bool| ListItem {
        name: name.into(),
        qty: "1".into(),
        cat: "chilled".into(),
        price,
        trip,
        checked,
        in_stock: false,
    };
    c.save_list_item(None, item("Skyr", 1, Some(2.5), false)).unwrap();
    c.save_list_item(None, item("Oats", 1, Some(1.75), false)).unwrap();
    c.save_list_item(None, item("Milk", 1, Some(1.0), true)).unwrap(); // already in the basket
    c.save_list_item(None, item("Tofu", 2, None, false)).unwrap();
    c.set_shop_day("2026-11-24", Some(1)).unwrap();
    let p = c.plan_ahead(7);
    assert_eq!(p.days[2].shop, Some(ShopDay { trip: 1, items: 2, cost: 4.25 }));
    assert!(p.days[0].shop.is_none());
    // Planning meals that day keeps the shopping mark, and the Log shows it on the day.
    c.add_plan_meal("2026-11-24", PlanMeal { slot: "Lunch".into(), name: "Bowl".into(), kcal: 500.0, prot: 30.0, ..Default::default() })
        .unwrap();
    assert_eq!(c.plan_ahead(7).days[2].shop.as_ref().map(|s| s.trip), Some(1));
    c.set_shop_day("2026-11-22", Some(2)).unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().shop, Some(ShopDay { trip: 2, items: 1, cost: 0.0 }));
    assert!(c.set_shop_day("2026-11-22", Some(3)).is_err());
    c.set_shop_day("2026-11-22", None).unwrap();
    assert!(c.day("2026-11-22").unwrap().shop.is_none());
    // Repeat last week carries the shopping day over.
    c.set_plan("2026-11-17", vec![PlanMeal { slot: "Lunch".into(), name: "Soup".into(), kcal: 300.0, prot: 10.0, ..Default::default() }])
        .unwrap();
    c.set_shop_day("2026-11-17", Some(1)).unwrap();
    c.repeat_last_week().unwrap();
    assert_eq!(c.store.get::<Plan>("2026-11-24").unwrap().shop, Some(1));
}

#[test]
fn plan_a_database_food() {
    let mut c = core("2026-11-22", "12:00");
    c.foods = Some(privatei_core::fooddb::sample_db());
    c.plan_food("2026-11-25", "Dinner", "usda:2", 250.0).unwrap();
    let day = &c.plan_ahead(7).days[3];
    assert_eq!(day.meals.len(), 1);
    let m = &day.meals[0];
    assert_eq!((m.slot.as_str(), m.kcal, m.prot, m.fiber), ("Dinner", 290.0, 22.5, Some(19.8)));
    assert!(m.name.ends_with("· 250 g"), "{}", m.name);
    assert!(c.plan_food("2026-11-25", "Dinner", "usda:999", 100.0).is_err());
    assert!(c.plan_food("2026-11-25", "Dinner", "usda:2", 0.0).is_err());
}

#[test]
fn swiss_pack_installs_dedupes_lookups_and_removes() {
    use privatei_core::fooddb::NewFood;
    use privatei_core::off;
    let dir = std::env::temp_dir().join(format!("privatei-pack-{}", privatei_core::store::new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut c = core("2026-11-22", "12:00");
    c.foods = Some(privatei_core::fooddb::sample_db());
    let pack = |name: &str| {
        format!(
            "{{\"pack\":\"off-ch\",\"date\":\"2026-11-23\"}}\n{{\"code\":\"7610000000011\",\"product_name\":\"{name}\",\"brands\":\"Migros\",\"nutriments\":{{\"energy-kcal_100g\":62,\"proteins_100g\":11}}}}\n"
        )
    };
    let dest = dir.join(off::PACK_FILE);
    let tmp = dir.join("a.part");
    off::pack_to_db(pack("Skyr nature").as_bytes(), &tmp).unwrap();
    c.install_off_pack(&tmp, &dest).unwrap();
    assert!(!tmp.exists() && dest.is_file());
    let hits = c.search_foods("skyr", 10);
    let hit = hits.iter().find(|f| f.source == off::PACK_SOURCE).unwrap();
    assert_eq!(hit.reference, "offch:7610000000011");
    c.add_food("2026-11-22", "Snack", &hit.reference, 150.0).unwrap();
    assert_eq!(c.day("2026-11-22").unwrap().kcal, 93.0);

    // An online lookup of a product the pack has returns the pack's entry, not a copy.
    let online = |code: &str, name: &str| NewFood {
        source_id: code.into(),
        name: name.into(),
        category: "Packaged product".into(),
        kcal100: 70.0,
        ..Default::default()
    };
    let got = c.save_off(vec![online("7610000000011", "Skyr nature (Migros)"), online("4000000000000", "Other skyr")]).unwrap();
    let refs: Vec<&str> = got.iter().map(|f| f.reference.as_str()).collect();
    assert_eq!(refs, ["offch:7610000000011", "off:4000000000000"]);
    let products = c.search_foods("skyr", 10).into_iter().filter(|f| f.category == "Packaged product").count();
    assert_eq!(products, 2, "no duplicate of the pack product");

    // Updating swaps the file in place.
    let tmp2 = dir.join("b.part");
    off::pack_to_db(pack("Skyr natur").as_bytes(), &tmp2).unwrap();
    c.install_off_pack(&tmp2, &dest).unwrap();
    assert_eq!(c.food_detail("offch:7610000000011").unwrap().name, "Skyr natur (Migros)");
    assert_eq!(c.food_sources().iter().filter(|s| s.id == off::PACK_SOURCE).count(), 1);
    // A missing temp file leaves the installed pack working.
    assert!(c.install_off_pack(&dir.join("missing.part"), &dest).is_err());
    assert!(c.food_detail("offch:7610000000011").is_some());

    // Removing it deletes the file; the online cache stays.
    c.remove_food_source(off::PACK_SOURCE).unwrap();
    assert!(!dest.exists());
    assert!(c.food_detail("offch:7610000000011").is_none());
    assert!(c.food_detail("off:4000000000000").is_some());
    assert_eq!(c.day("2026-11-22").unwrap().kcal, 93.0, "the log keeps its numbers");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn generic_names_rank_and_match_generic_foods_before_products() {
    use privatei_core::ai::{AiAnswer, Provider};
    use privatei_core::fooddb::NewFood;
    let mut c = core("2026-11-22", "16:00");
    c.foods = Some(privatei_core::fooddb::sample_db());
    let product = |code: &str, name: &str| NewFood {
        source_id: code.into(),
        name: name.into(),
        category: "Packaged product".into(),
        kcal100: 80.0,
        prot100: 10.0,
        ..Default::default()
    };
    let products: Vec<NewFood> = (0..12).map(|i| product(&format!("76100000000{i:02}"), &format!("Lentils snack {i} (Coop)"))).collect();
    c.save_off(products).unwrap();
    c.save_off(vec![product("7610000000999", "Protein pudding (Ehrmann)")]).unwrap();
    // Search: the generic food first, and a third of the list kept for products.
    let hits = c.search_foods("lentils", 9);
    assert_eq!(hits[0].reference, "usda:2");
    assert_eq!(hits.iter().filter(|h| h.source == "off").count(), 8, "only one generic lentil food, so products fill the rest");
    assert_eq!(
        c.foods.as_ref().unwrap().search("lentils", 3).iter().map(|h| h.source.as_str()).collect::<Vec<_>>(),
        ["usda", "off", "off"]
    );
    // Work out: a generic name matches the generic food, never a packaged product.
    let p = c.prepare_estimate("200 g lentils and a protein pudding").unwrap();
    let items = serde_json::json!({"items": [
        {"name": "Lentils", "food": "lentils", "grams": 200, "kcal": 300, "prot": 20, "est": false, "meal": "Snack"},
        {"name": "Protein pudding", "food": "protein pudding", "grams": 200, "kcal": 160, "prot": 20, "est": false, "meal": "Snack"}
    ], "note": ""});
    let v = c.finish_estimate(p, Some(Ok(AiAnswer { provider: Provider::Gemini, value: items })));
    let lentils = v.items.iter().find(|r| r.item.name.starts_with("Lentils")).unwrap();
    assert_eq!(lentils.item.food_ref.as_deref(), Some("usda:2"));
    let pudding = v.items.iter().find(|r| r.item.name.starts_with("Protein")).unwrap();
    assert!(pudding.item.food_ref.is_none(), "not matched to the Ehrmann product");
    assert_eq!(pudding.item.kcal, 160.0);
}
