//! Private-I desktop: thin Tauri commands. Every calculation lives in privatei-core.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use privatei_core::ai::{self, RuleChange};
use privatei_core::app::*;
use privatei_core::food::Item;
use privatei_core::model::{ListItem, PlanMeal, Settings, Stock};
use privatei_core::Core;
use std::sync::Mutex;
use tauri::{Manager, State};

struct AppState(Mutex<Core>);

type R<T> = std::result::Result<T, String>;

fn with<T>(s: &State<AppState>, f: impl FnOnce(&mut Core) -> R<T>) -> R<T> {
    let mut c = s.0.lock().map_err(|_| "The app hit an internal error. Please restart it.".to_string())?;
    f(&mut c)
}

/// Run slow work (AI calls) off the main thread and without holding the lock.
async fn off_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| format!("Background task failed: {e}"))
}

// ---------- Today ----------

#[tauri::command]
fn today() -> String {
    privatei_core::dates::today()
}

#[tauri::command]
fn get_day(s: State<AppState>, date: String) -> R<DayView> {
    with(&s, |c| c.day(&date))
}

#[tauri::command]
fn add_log(s: State<AppState>, date: String, entry: LogInput) -> R<String> {
    with(&s, |c| c.add_log(&date, entry))
}

#[tauri::command]
fn update_log(s: State<AppState>, id: String, entry: LogInput) -> R<()> {
    with(&s, |c| c.update_log(&id, entry))
}

#[tauri::command]
fn update_plan_meal(s: State<AppState>, date: String, idx: usize, name: Option<String>, meal: PlanMeal) -> R<()> {
    with(&s, |c| c.update_plan_meal(&date, idx, name.as_deref(), meal))
}

#[tauri::command]
fn update_workout(s: State<AppState>, id: String, kind: String, mins: f64, kcal: Option<f64>, name: String) -> R<()> {
    with(&s, |c| c.update_workout(&id, &kind, mins, kcal, &name))
}

#[tauri::command]
fn save_items(s: State<AppState>, date: String, items: Vec<Item>) -> R<usize> {
    with(&s, |c| c.save_items(&date, items))
}

#[tauri::command]
fn delete_log(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.delete_log(&id))
}

#[tauri::command]
fn set_sleep(s: State<AppState>, date: String, bed: String, wake: String) -> R<()> {
    with(&s, |c| c.set_sleep(&date, &bed, &wake))
}

#[tauri::command]
fn clear_sleep(s: State<AppState>, date: String) -> R<()> {
    with(&s, |c| c.clear_sleep(&date))
}

#[tauri::command]
fn toggle_plan(s: State<AppState>, date: String, idx: usize, name: Option<String>) -> R<bool> {
    with(&s, |c| c.toggle_plan(&date, idx, name.as_deref()))
}

#[tauri::command]
fn set_plan(s: State<AppState>, date: String, meals: Vec<PlanMeal>) -> R<()> {
    with(&s, |c| c.set_plan(&date, meals))
}

#[tauri::command]
fn copy_yesterday(s: State<AppState>, date: String) -> R<usize> {
    with(&s, |c| c.copy_yesterday(&date))
}

#[tauri::command]
fn recent_foods(s: State<AppState>) -> R<Vec<RecentFood>> {
    with(&s, |c| Ok(c.recent_foods()))
}

// ---------- Add ----------

#[tauri::command]
async fn estimate_food(s: State<'_, AppState>, text: String) -> R<EstimateView> {
    let pending = with(&s, |c| c.prepare_estimate(&text))?;
    let answer = match pending.ai.clone() {
        Some((cfg, req)) => Some(off_thread(move || ai::ask(&cfg, &req)).await?),
        None => None,
    };
    with(&s, |c| Ok(c.finish_estimate(pending, answer)))
}

#[tauri::command]
async fn estimate_photo(s: State<'_, AppState>, data: String, ext: String, note: String) -> R<EstimateView> {
    let bytes = ai::base64_decode(&data).ok_or("That photo couldn't be read.")?;
    let pending = with(&s, |c| c.prepare_photo(bytes, &ext, &note))?;
    let (cfg, req) = pending.ai.clone().ok_or("No AI request")?;
    let answer = off_thread(move || ai::ask(&cfg, &req)).await?;
    with(&s, |c| Ok(c.finish_estimate(pending, Some(answer))))
}

// ---------- Food database ----------

#[tauri::command]
fn search_foods(s: State<AppState>, query: String) -> R<Vec<privatei_core::fooddb::DbFood>> {
    with(&s, |c| Ok(c.search_foods(&query, 30)))
}

#[tauri::command]
fn food_detail(s: State<AppState>, reference: String) -> R<Option<privatei_core::fooddb::DbFood>> {
    with(&s, |c| Ok(c.food_detail(&reference)))
}

#[tauri::command]
fn add_food(s: State<AppState>, date: String, meal: String, reference: String, grams: f64) -> R<String> {
    with(&s, |c| c.add_food(&date, &meal, &reference, grams))
}

#[tauri::command]
fn ingredient(s: State<AppState>, reference: String, grams: f64) -> R<privatei_core::model::Ingredient> {
    with(&s, |c| c.ingredient(&reference, grams))
}

#[tauri::command]
fn recipe_preview(s: State<AppState>, recipe: RecipeInput) -> R<RecipeView> {
    with(&s, |c| c.recipe_preview(recipe))
}

#[tauri::command]
fn save_recipe(s: State<AppState>, id: Option<String>, recipe: RecipeInput) -> R<String> {
    with(&s, |c| c.save_recipe(id, recipe))
}

#[tauri::command]
fn get_recipes(s: State<AppState>) -> R<Vec<RecipeView>> {
    with(&s, |c| Ok(c.recipes()))
}

#[tauri::command]
fn delete_recipe(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.delete_recipe(&id))
}

#[tauri::command]
fn food_sources(s: State<AppState>) -> R<Vec<privatei_core::fooddb::SourceInfo>> {
    with(&s, |c| Ok(c.food_sources()))
}

#[tauri::command]
async fn download_swiss(s: State<'_, AppState>) -> R<usize> {
    let (bytes, version) = off_thread(privatei_core::importers::download_swiss).await??;
    with(&s, |c| c.import_swiss(&bytes, &version))
}

#[tauri::command]
async fn search_off(s: State<'_, AppState>, query: String) -> R<Vec<privatei_core::fooddb::DbFood>> {
    let foods = off_thread(move || privatei_core::off::fetch(&query, privatei_core::off::BASE)).await??;
    with(&s, |c| c.save_off(foods))
}

#[tauri::command]
fn remove_food_source(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.remove_food_source(&id))
}

#[tauri::command]
fn quick_fill(s: State<AppState>, name: String, grams: Option<f64>) -> R<Option<QuickFill>> {
    with(&s, |c| Ok(c.quick(&name, grams)))
}

#[tauri::command]
fn food_names(s: State<AppState>) -> R<Vec<String>> {
    with(&s, |c| Ok(c.food_names()))
}

// ---------- Meds ----------

#[tauri::command]
fn get_meds(s: State<AppState>) -> R<MedsView> {
    with(&s, |c| Ok(c.meds_view()))
}

#[tauri::command]
fn save_med(s: State<AppState>, med: MedInput) -> R<SaveMedResult> {
    with(&s, |c| c.save_med(med))
}

#[tauri::command]
fn stop_med(s: State<AppState>, group_id: String) -> R<()> {
    with(&s, |c| c.stop_med(&group_id))
}

#[tauri::command]
fn erase_med(s: State<AppState>, group_id: String) -> R<usize> {
    with(&s, |c| c.erase_med(&group_id))
}

#[tauri::command]
fn toggle_dose(s: State<AppState>, date: String, med_id: String) -> R<bool> {
    with(&s, |c| c.toggle_dose(&date, &med_id))
}

// ---------- Training ----------

#[tauri::command]
fn burn_estimate(s: State<AppState>, kind: String, mins: f64) -> R<f64> {
    with(&s, |c| Ok(c.burn_estimate(&kind, mins)))
}

#[tauri::command]
fn add_workout(s: State<AppState>, date: String, kind: String, mins: f64, kcal: Option<f64>, name: String) -> R<String> {
    with(&s, |c| c.add_workout(&date, &kind, mins, kcal, &name))
}

#[tauri::command]
fn recent_workouts(s: State<AppState>, limit: usize) -> R<Vec<WorkoutEntry>> {
    with(&s, |c| Ok(c.recent_workouts(limit)))
}

#[tauri::command]
fn delete_workout(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.delete_workout(&id))
}

#[tauri::command]
fn import_workouts(s: State<AppState>, text: String) -> R<ImportResult> {
    with(&s, |c| c.import_workouts(&text))
}

#[tauri::command]
fn set_add_burn(s: State<AppState>, on: bool) -> R<()> {
    with(&s, |c| c.set_add_burn(on))
}

// ---------- Stock ----------

#[tauri::command]
fn get_stock(s: State<AppState>) -> R<StockView> {
    with(&s, |c| Ok(c.stock_view()))
}

#[tauri::command]
fn save_stock(s: State<AppState>, id: Option<String>, item: Stock) -> R<String> {
    with(&s, |c| c.save_stock(id, item))
}

#[tauri::command]
fn delete_stock(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.delete_stock(&id))
}

// ---------- Shop ----------

#[tauri::command]
fn get_shop(s: State<AppState>, trip: u8) -> R<ShopView> {
    with(&s, |c| Ok(c.shop_view(trip)))
}

#[tauri::command]
fn save_list_item(s: State<AppState>, id: Option<String>, item: ListItem) -> R<String> {
    with(&s, |c| c.save_list_item(id, item))
}

#[tauri::command]
fn toggle_list_item(s: State<AppState>, id: String) -> R<bool> {
    with(&s, |c| c.toggle_list_item(&id))
}

#[tauri::command]
fn delete_list_item(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.delete_list_item(&id))
}

#[tauri::command]
fn check_have(s: State<AppState>) -> R<Vec<HaveMatch>> {
    with(&s, |c| Ok(c.check_have()))
}

#[tauri::command]
fn resolve_have(s: State<AppState>, list_id: String, stock_id: String, still_there: bool) -> R<()> {
    with(&s, |c| c.resolve_have(&list_id, &stock_id, still_there))
}

#[tauri::command]
fn move_back(s: State<AppState>, list_id: String) -> R<()> {
    with(&s, |c| c.move_back(&list_id))
}

#[tauri::command]
fn bought(s: State<AppState>, trip: u8) -> R<usize> {
    with(&s, |c| c.bought(trip))
}

// ---------- Week ----------

#[tauri::command]
fn get_week(s: State<AppState>) -> R<WeekView> {
    with(&s, |c| Ok(c.week_view()))
}

#[tauri::command]
fn repeat_last_week(s: State<AppState>) -> R<usize> {
    with(&s, |c| c.repeat_last_week())
}

#[tauri::command]
fn get_history(s: State<AppState>, range: Option<usize>) -> R<History> {
    with(&s, |c| Ok(c.history_range(range.unwrap_or(60))))
}

#[tauri::command]
fn get_plan_ahead(s: State<AppState>, days: usize) -> R<PlanAhead> {
    with(&s, |c| Ok(c.plan_ahead(days)))
}

#[tauri::command]
fn add_plan_meal(s: State<AppState>, date: String, meal: PlanMeal) -> R<()> {
    with(&s, |c| c.add_plan_meal(&date, meal))
}

#[tauri::command]
fn remove_plan_meal(s: State<AppState>, date: String, idx: usize, name: Option<String>) -> R<()> {
    with(&s, |c| c.remove_plan_meal(&date, idx, name.as_deref()))
}

#[tauri::command]
fn training_summary(s: State<AppState>) -> R<TrainingSummary> {
    with(&s, |c| Ok(c.training_summary()))
}

// ---------- Settings ----------

#[tauri::command]
fn get_settings(s: State<AppState>) -> R<Settings> {
    with(&s, |c| Ok(c.settings()))
}

#[tauri::command]
fn save_settings(s: State<AppState>, settings: Settings) -> R<()> {
    with(&s, |c| c.save_settings(settings))
}

#[tauri::command]
fn get_ai_settings(s: State<AppState>) -> R<AiSettingsView> {
    with(&s, |c| Ok(c.ai_settings_view()))
}

#[tauri::command]
fn save_ai_settings(s: State<AppState>, ai: AiSettingsInput) -> R<()> {
    with(&s, |c| c.save_ai_settings(ai))
}

#[tauri::command]
async fn test_ai(s: State<'_, AppState>) -> R<String> {
    let (cfg, req) = with(&s, |c| Ok(c.prepare_test_ai()))?;
    let ans = off_thread(move || ai::ask(&cfg, &req)).await??;
    Ok(format!("{} answered. AI is working.", ans.provider.label()))
}

#[tauri::command]
fn learned_foods(s: State<AppState>) -> R<Vec<LearnedFood>> {
    with(&s, |c| Ok(c.learned_foods()))
}

#[tauri::command]
fn delete_learned(s: State<AppState>, id: String) -> R<()> {
    with(&s, |c| c.delete_learned(&id))
}

// ---------- Rules ----------

#[tauri::command]
fn get_rules(s: State<AppState>) -> R<String> {
    with(&s, |c| Ok(c.rules()))
}

#[tauri::command]
async fn rules_chat(s: State<'_, AppState>, message: String) -> R<RulesReply> {
    let (cfg, req) = with(&s, |c| c.prepare_rules_chat(&message))?;
    let ans = off_thread(move || ai::ask(&cfg, &req)).await??;
    Ok(Core::finish_rules_chat(ans))
}

#[tauri::command]
fn apply_rule_change(s: State<AppState>, change: RuleChange) -> R<String> {
    with(&s, |c| c.apply_rule_change(&change))
}

// ---------- Backup ----------

#[tauri::command]
fn export_to(s: State<AppState>, path: String) -> R<()> {
    let text = with(&s, |c| Ok(c.export_json()))?;
    std::fs::write(&path, text).map_err(|e| format!("Couldn't save the backup: {e}"))
}

#[tauri::command]
fn import_from(s: State<AppState>, path: String) -> R<usize> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("Couldn't open that file: {e}"))?;
    if meta.len() > 100 * 1024 * 1024 {
        return Err("That file is too large to be a backup.".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't read that file: {e}"))?;
    with(&s, |c| c.import_json(&text))
}

fn device_id(dir: &std::path::Path) -> String {
    let p = dir.join("device-id");
    if let Ok(id) = std::fs::read_to_string(&p) {
        if !id.trim().is_empty() {
            return id.trim().to_string();
        }
    }
    let id = format!("mac-{}", privatei_core::store::new_id());
    let _ = std::fs::write(&p, &id);
    id
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let cache = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data)?;
            std::fs::create_dir_all(&cache)?;
            let mut core = Core::open(&data, &cache, &device_id(&data)).map_err(std::io::Error::other)?;
            // The shipped USDA database (read-only) plus a writable one for downloads.
            let bundled = app.path().resolve("resources/foods.sqlite", tauri::path::BaseDirectory::Resource).ok();
            match privatei_core::fooddb::FoodDb::open(bundled.as_deref(), &data.join("foods-user.sqlite")) {
                Ok(db) => core.foods = Some(db),
                Err(e) => eprintln!("Private-I: food database unavailable: {e}"),
            }
            if core.store.skipped > 0 {
                eprintln!("Private-I: skipped {} unreadable rows at startup", core.store.skipped);
            }
            app.manage(AppState(Mutex::new(core)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            today,
            get_day,
            add_log,
            save_items,
            update_log,
            update_plan_meal,
            update_workout,
            delete_log,
            toggle_plan,
            set_sleep,
            clear_sleep,
            set_plan,
            copy_yesterday,
            recent_foods,
            estimate_food,
            estimate_photo,
            quick_fill,
            search_foods,
            food_detail,
            add_food,
            food_sources,
            ingredient,
            recipe_preview,
            save_recipe,
            get_recipes,
            delete_recipe,
            download_swiss,
            search_off,
            remove_food_source,
            food_names,
            get_meds,
            save_med,
            stop_med,
            erase_med,
            toggle_dose,
            burn_estimate,
            add_workout,
            delete_workout,
            recent_workouts,
            import_workouts,
            set_add_burn,
            get_stock,
            save_stock,
            delete_stock,
            get_shop,
            save_list_item,
            toggle_list_item,
            delete_list_item,
            check_have,
            resolve_have,
            move_back,
            bought,
            get_week,
            repeat_last_week,
            get_history,
            get_plan_ahead,
            add_plan_meal,
            remove_plan_meal,
            training_summary,
            get_settings,
            save_settings,
            get_ai_settings,
            save_ai_settings,
            test_ai,
            learned_foods,
            delete_learned,
            get_rules,
            rules_chat,
            apply_rule_change,
            export_to,
            import_from,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Private-I");
}
