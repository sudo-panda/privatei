//! AI providers: the user's installed Claude Code CLI (desktop, feature `cli`)
//! and Gemini over HTTPS. Never an Anthropic API key, never claude.ai endpoints.

use crate::food::{self, Item};
use crate::model::{meal_for_time, valid_model_name, AiSettings, Settings, MEALS};
use crate::nutrients;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

pub const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-flash-latest";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Gemini,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Provider::Claude => "Claude",
            Provider::Gemini => "Gemini",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub order: Vec<Provider>,
    pub gemini_key: String,
    pub gemini_model: String,
    pub gemini_base: String,
    pub claude_model: String,
    /// Override for the `claude` binary; otherwise searched in the usual places.
    pub claude_path: Option<PathBuf>,
    /// Scratch folders for the CLI are created under here (the app cache).
    pub scratch_root: PathBuf,
    pub timeout: Duration,
}

impl AiConfig {
    pub fn from_settings(s: &AiSettings, scratch_root: PathBuf) -> AiConfig {
        let mut order: Vec<Provider> = s
            .order
            .iter()
            .filter_map(|p| match p.as_str() {
                "claude" => Some(Provider::Claude),
                "gemini" => Some(Provider::Gemini),
                _ => None,
            })
            .collect();
        order.dedup();
        if order.is_empty() {
            order = vec![Provider::Claude, Provider::Gemini];
        }
        let gm = s.gemini_model.trim();
        AiConfig {
            order,
            gemini_key: s.gemini_key.trim().to_string(),
            gemini_model: if gm.is_empty() { DEFAULT_GEMINI_MODEL.into() } else { gm.into() },
            gemini_base: GEMINI_BASE.into(),
            claude_model: s.claude_model.trim().to_string(),
            claude_path: Some(s.claude_path.trim()).filter(|p| !p.is_empty()).map(PathBuf::from),
            scratch_root,
            timeout: Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
    /// jpg, png, webp, heic …
    pub ext: String,
}

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub prompt: String,
    pub image: Option<Image>,
}

#[derive(Debug, Clone)]
pub struct AiAnswer {
    pub provider: Provider,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiError {
    /// Not set up on this device; skipped quietly.
    NotSetUp,
    RateLimited,
    Failed(String),
}

/// Ask the providers in order; the first good JSON answer wins.
pub fn ask(cfg: &AiConfig, req: &AiRequest) -> Result<AiAnswer, String> {
    let mut errors: Vec<(Provider, AiError)> = vec![];
    for &p in &cfg.order {
        let r = match p {
            Provider::Claude => claude(cfg, req),
            Provider::Gemini => gemini(cfg, req),
        };
        match r {
            Ok(value) => return Ok(AiAnswer { provider: p, value }),
            Err(e) => errors.push((p, e)),
        }
    }
    Err(combine_errors(&errors))
}

/// One plain-language message for every failure.
pub fn combine_errors(errors: &[(Provider, AiError)]) -> String {
    let real: Vec<&(Provider, AiError)> = errors.iter().filter(|(_, e)| *e != AiError::NotSetUp).collect();
    if real.is_empty() {
        return "No AI is set up. Install Claude Code or add a Gemini key in Settings.".into();
    }
    if real.iter().all(|(_, e)| *e == AiError::RateLimited) {
        return "The AI is rate limited right now. Try again in a minute.".into();
    }
    let parts: Vec<String> = real
        .iter()
        .map(|(p, e)| match e {
            AiError::RateLimited => format!("{} is rate limited", p.label()),
            AiError::Failed(m) => format!("{}: {}", p.label(), m),
            AiError::NotSetUp => unreachable!(),
        })
        .collect();
    format!("The AI couldn't answer. {}.", parts.join("; "))
}

/// Pull a JSON object out of model text (```json fences, text around it).
pub fn extract_json(text: &str) -> Option<Value> {
    let t = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        if v.is_object() {
            return Some(v);
        }
        if let Value::String(s) = v {
            return extract_json(&s);
        }
    }
    if let Some(start) = t.find("```") {
        let rest = &t[start + 3..];
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        if let Some(end) = rest.find("```") {
            if let Ok(v) = serde_json::from_str::<Value>(rest[..end].trim()) {
                if v.is_object() {
                    return Some(v);
                }
            }
        }
    }
    // Try every '{' from the left against the last '}'.
    let end = t.rfind('}')?;
    for (i, _) in t.match_indices('{') {
        if i >= end {
            break;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&t[i..=end]) {
            if v.is_object() {
                return Some(v);
            }
        }
    }
    None
}

// ---------- Claude Code CLI ----------

/// Where a Mac app may find `claude` (apps don't inherit the Terminal PATH).
pub fn claude_candidates(home: &std::path::Path) -> Vec<PathBuf> {
    let mut v = vec![
        home.join(".local/bin/claude"),
        home.join(".claude/local/claude"),
        home.join(".homebrew/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        home.join(".npm-global/bin/claude"),
    ];
    if let Ok(rd) = std::fs::read_dir(home.join(".nvm/versions/node")) {
        let mut nvm: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path().join("bin/claude")).collect();
        nvm.sort();
        nvm.reverse(); // newest node first
        v.extend(nvm);
    }
    v
}

pub fn find_claude(cfg: &AiConfig) -> Option<PathBuf> {
    if let Some(p) = &cfg.claude_path {
        return p.is_file().then(|| p.clone());
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    claude_candidates(&home).into_iter().find(|p| p.is_file())
}

/// The exact argument list passed to `claude` (no shell involved).
pub fn claude_args(prompt: &str, model: &str) -> Vec<String> {
    let mut a: Vec<String> = [
        "-p",
        prompt,
        "--output-format",
        "json",
        "--max-turns",
        "4",
        "--allowedTools",
        "Read",
        "--disallowedTools",
        "Bash",
        "Edit",
        "Write",
        "MultiEdit",
        "NotebookEdit",
        "WebFetch",
        "WebSearch",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    if !model.is_empty() {
        a.push("--model".into());
        a.push(model.into());
    }
    a
}

/// Parse the CLI output: the last JSON line `{type:"result", is_error, result}`.
pub fn parse_claude_output(stdout: &str) -> Result<Value, AiError> {
    let line = stdout
        .lines()
        .rev()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .find(|v| v.get("type").and_then(Value::as_str) == Some("result"))
        .or_else(|| serde_json::from_str::<Value>(stdout.trim()).ok().filter(|v| v.get("type").and_then(Value::as_str) == Some("result")))
        .ok_or_else(|| AiError::Failed("no answer from Claude".into()))?;
    let result = line.get("result").and_then(Value::as_str).unwrap_or("");
    if line.get("is_error").and_then(Value::as_bool).unwrap_or(false) {
        let low = result.to_lowercase();
        if low.contains("rate limit") || low.contains("usage limit") || low.contains("limit reached") {
            return Err(AiError::RateLimited);
        }
        return Err(AiError::Failed(if result.is_empty() { "Claude reported an error".into() } else { short(result) }));
    }
    extract_json(result).ok_or_else(|| AiError::Failed("Claude's answer wasn't JSON".into()))
}

fn short(s: &str) -> String {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.chars().take(160).collect()
}

#[cfg(feature = "cli")]
fn claude(cfg: &AiConfig, req: &AiRequest) -> Result<Value, AiError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    let bin = find_claude(cfg).ok_or(AiError::NotSetUp)?;
    if !valid_model_name(&cfg.claude_model) {
        return Err(AiError::Failed("the Claude model name is not valid".into()));
    }
    let dir = cfg.scratch_root.join(format!("ai-{}", crate::store::new_id()));
    std::fs::create_dir_all(&dir).map_err(|e| AiError::Failed(format!("can't make a scratch folder ({e})")))?;
    let mut prompt = req.prompt.clone();
    if let Some(img) = &req.image {
        let ext = safe_ext(&img.ext);
        let file = dir.join(format!("meal.{ext}"));
        std::fs::write(&file, &img.bytes).map_err(|e| AiError::Failed(format!("can't save the photo ({e})")))?;
        prompt = format!("Use the Read tool to look at the photo of the meal in the file {}. {}", file.display(), prompt);
    }
    let result = (|| {
        // "Text file busy" (ETXTBSY) can happen briefly if the binary was just written
        // (e.g. mid-update, or a test's fake script); retry a few times before giving up.
        let mut attempt = 0;
        let mut child = loop {
            let spawned = Command::new(&bin)
                .args(claude_args(&prompt, &cfg.claude_model))
                .current_dir(&dir)
                .env("PATH", extended_path(&bin))
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            match spawned {
                Err(e) if e.raw_os_error() == Some(26) && attempt < 5 => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_millis(50 * attempt));
                }
                Err(e) => return Err(AiError::Failed(format!("couldn't start claude ({e})"))),
                Ok(c) => break c,
            }
        };
        let mut out = child.stdout.take().unwrap();
        let mut err = child.stderr.take().unwrap();
        let t_out = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        });
        let t_err = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        });
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if start.elapsed() > cfg.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AiError::Failed(format!("Claude took longer than {} seconds", cfg.timeout.as_secs())));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => return Err(AiError::Failed(e.to_string())),
            }
        };
        let stdout = t_out.join().unwrap_or_default();
        let stderr = t_err.join().unwrap_or_default();
        match parse_claude_output(&stdout) {
            Ok(v) => Ok(v),
            Err(e) if !status.success() && stdout.trim().is_empty() => {
                let low = stderr.to_lowercase();
                if low.contains("rate limit") || low.contains("usage limit") {
                    Err(AiError::RateLimited)
                } else if low.contains("log in") || low.contains("login") || low.contains("auth") {
                    Err(AiError::Failed("Claude Code isn't logged in (run `claude` once in Terminal)".into()))
                } else if stderr.trim().is_empty() {
                    Err(e)
                } else {
                    Err(AiError::Failed(short(&stderr)))
                }
            }
            Err(e) => Err(e),
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(not(feature = "cli"))]
fn claude(_cfg: &AiConfig, _req: &AiRequest) -> Result<Value, AiError> {
    Err(AiError::NotSetUp)
}

/// `claude` is often a node script: make sure its own folder (where `node` lives for nvm) is on PATH.
#[cfg(feature = "cli")]
fn extended_path(bin: &std::path::Path) -> std::ffi::OsString {
    let mut dirs: Vec<PathBuf> = vec![];
    if let Some(d) = bin.parent() {
        dirs.push(d.to_path_buf());
    }
    if let Ok(real) = std::fs::canonicalize(bin) {
        if let Some(d) = real.parent() {
            dirs.push(d.to_path_buf());
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".homebrew/bin"));
    }
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
        dirs.push(PathBuf::from(d));
    }
    if let Some(p) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&p));
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

fn safe_ext(ext: &str) -> String {
    let e: String = ext.trim_start_matches('.').to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).take(5).collect();
    if e.is_empty() {
        "jpg".into()
    } else {
        e
    }
}

fn mime(ext: &str) -> &'static str {
    match safe_ext(ext).as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "gif" => "image/gif",
        _ => "image/jpeg",
    }
}

// ---------- Gemini ----------

fn gemini(cfg: &AiConfig, req: &AiRequest) -> Result<Value, AiError> {
    if cfg.gemini_key.is_empty() {
        return Err(AiError::NotSetUp);
    }
    if !valid_model_name(&cfg.gemini_model) || cfg.gemini_model.is_empty() {
        return Err(AiError::Failed("the Gemini model name is not valid".into()));
    }
    let mut parts = vec![json!({ "text": req.prompt })];
    if let Some(img) = &req.image {
        parts.push(json!({ "inline_data": { "mime_type": mime(&img.ext), "data": base64(&img.bytes) } }));
    }
    let body = json!({
        "contents": [{ "role": "user", "parts": parts }],
        "generationConfig": { "responseMimeType": "application/json" }
    });
    let url = format!("{}/models/{}:generateContent", cfg.gemini_base.trim_end_matches('/'), cfg.gemini_model);
    let client = reqwest::blocking::Client::builder().timeout(cfg.timeout).build().map_err(|e| AiError::Failed(e.to_string()))?;
    let resp =
        client.post(url).header("x-goog-api-key", &cfg.gemini_key).json(&body).send().map_err(|e| {
            AiError::Failed(if e.is_timeout() { "Gemini timed out".into() } else { "couldn't reach Gemini (offline?)".into() })
        })?;
    let status = resp.status().as_u16();
    match status {
        200 => {}
        429 => return Err(AiError::RateLimited),
        400 | 401 | 403 | 404 => return Err(AiError::Failed("check the Gemini key or model in Settings".into())),
        s => return Err(AiError::Failed(format!("Gemini returned an error ({s})"))),
    }
    let v: Value = resp.json().map_err(|_| AiError::Failed("Gemini's reply wasn't readable".into()))?;
    let text: String = v["candidates"][0]["content"]["parts"]
        .as_array()
        .map(|a| a.iter().filter_map(|p| p["text"].as_str()).collect())
        .unwrap_or_default();
    extract_json(&text).ok_or_else(|| AiError::Failed("Gemini's answer wasn't JSON".into()))
}

/// Standard base64 (padded).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Decode standard base64 (used for photos coming from the UI).
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.split_once("base64,").map(|x| x.1).unwrap_or(s);
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        } as u32;
        buf = buf << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

// ---------- Prompts and cleaning ----------

pub fn food_prompt(s: &Settings, rules: &str, user_text: &str) -> String {
    let w = food::fmt_num(s.weight_kg);
    let k = food::fmt_num(s.kcal_target);
    let p = s.prot_floor.round() as i64;
    let rules = if rules.trim().is_empty() { "none".to_string() } else { rules.trim().to_string() };
    format!(
        "You are logging food for a 26-year-old man cutting from {w}kg toward 65kg in Zurich. Targets: ~{k} kcal/day, protein {p}-{p2} g/day (protect protein first), saturated fat under 17 g/day. Never include: quark, tuna, lentils, raw tomato (cooked is fine), lettuce. Edamame only as a garnish, 6-8 beans - it bloats him. Dressings are lemon juice plus Thomy Inferno mustard, never oil. When food was roasted dry and weighed AFTER cooking, use the post-roast concentrated values, not raw. Per 100 g (kcal/protein/fiber): {table}. Mark est:true on any line whose weight you estimated rather than were told. Brand or product names (Migros, Coop, Da Emilio, Anna's Best, M-Classic, Betty Bossi, …) are Swiss supermarket products: use their label values per 100 g if you know them, and count the whole pack if he says he ate one. Extra house rules he has set: {rules}. He reports: {said}. For each item also estimate, where you reasonably can, these nutrients for the portion: fiber, carbs, sugar, addedSugar, fat, satFat (g); sodium, potassium, calcium, magnesium, iron, zinc (mg); vitA, folate, b12, vitD, iodine (µg); vitC (mg); caffeine (mg). Leave a key out rather than guessing wildly. Return JSON only: {{\"items\":[{{\"name\":string,\"grams\":number|null,\"kcal\":number,\"prot\":number,\"fiber\":number,\"addedSugar\":number,\"satFat\":number,\"sodium\":number,\"...\":number,\"est\":boolean,\"meal\":\"Breakfast\"|\"Lunch\"|\"Snack\"|\"Dinner\"}}],\"note\":string}}. note is at most one short sentence, or empty. Estimate grams yourself rather than asking.",
        p2 = p + 20,
        table = food::table_text(),
        said = serde_json::to_string(user_text).unwrap_or_default(),
    )
}

pub fn rules_prompt(rules: &str, message: &str) -> String {
    format!(
        "You help a man keep the house rules of his personal food tracker short and clear. The rules are notes the food logger must follow. Current rules (one per line): {rules}. He says: {said}. Reply in at most 2 sentences. If he wants a rule added, changed or removed, propose it: \"add\" is the new rule line (or empty), \"remove\" is an existing line to delete exactly as written (or null). Return JSON only: {{\"reply\":string,\"change\":{{\"add\":string,\"remove\":string|null}}|null}}.",
        rules = serde_json::to_string(rules).unwrap_or_default(),
        said = serde_json::to_string(message).unwrap_or_default(),
    )
}

pub const TEST_PROMPT: &str = "This is a connection test. Return JSON only: {\"ok\":true}";

fn clamp(v: &Value, max: f64) -> f64 {
    let n = match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    };
    if n.is_finite() {
        n.clamp(0.0, max)
    } else {
        0.0
    }
}

pub fn clip(s: &str, n: usize) -> String {
    s.trim().chars().filter(|c| !c.is_control()).take(n).collect::<String>().trim().to_string()
}

/// Clean an AI food answer: ≤30 items, names ≤80 chars, kcal 0–5000, protein 0–500 g, valid meal.
pub fn clean_food(v: &Value, now_hhmm: &str) -> (Vec<Item>, String) {
    let guess = meal_for_time(now_hhmm);
    let items = v
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|i| i.is_object())
                .filter_map(|i| {
                    let name = clip(i.get("name").and_then(Value::as_str).unwrap_or(""), 80);
                    if name.is_empty() {
                        return None;
                    }
                    let grams = i.get("grams").filter(|g| !g.is_null()).map(|g| clamp(g, 5000.0)).filter(|g| *g > 0.0);
                    let meal = i.get("meal").and_then(Value::as_str).filter(|m| MEALS.contains(m)).unwrap_or(guess);
                    let num = |k: &str, max: f64| i.get(k).filter(|v| !v.is_null()).map(|v| clamp(v, max));
                    let fiber = num("fiber", 300.0).map(food::round1);
                    let mut nut = nutrients::from_json(i);
                    nut.remove("fiber");
                    let nut = (!nut.is_empty()).then_some(nut);
                    Some(Item {
                        name,
                        grams: grams.map(food::round1),
                        kcal: clamp(i.get("kcal").unwrap_or(&Value::Null), 5000.0).round(),
                        prot: food::round1(clamp(i.get("prot").unwrap_or(&Value::Null), 500.0)),
                        est: i.get("est").and_then(Value::as_bool).unwrap_or(true),
                        meal: meal.to_string(),
                        fiber,
                        nut,
                    })
                })
                .take(30)
                .collect()
        })
        .unwrap_or_default();
    let note = clip(v.get("note").and_then(Value::as_str).unwrap_or(""), 200);
    (items, note)
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct RuleChange {
    pub add: String,
    pub remove: Option<String>,
}

/// At most two sentences.
pub fn two_sentences(s: &str) -> String {
    let s = clip(s, 600);
    let mut out = String::new();
    let mut count = 0;
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        out.push(*c);
        if matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace()) {
            count += 1;
            if count == 2 {
                break;
            }
        }
    }
    out.trim().to_string()
}

pub fn clean_rules_reply(v: &Value) -> (String, Option<RuleChange>) {
    let reply = two_sentences(v.get("reply").and_then(Value::as_str).unwrap_or(""));
    let change = v.get("change").filter(|c| c.is_object()).and_then(|c| {
        let add = clip(c.get("add").and_then(Value::as_str).unwrap_or(""), 200);
        let remove = c.get("remove").and_then(Value::as_str).map(|r| clip(r, 200)).filter(|r| !r.is_empty());
        (!add.is_empty() || remove.is_some()).then_some(RuleChange { add, remove })
    });
    (reply, change)
}

/// Apply a proposed change to the rules text (one rule per line).
pub fn apply_rule_change(rules: &str, c: &RuleChange) -> String {
    let mut lines: Vec<String> = rules.lines().map(|l| l.trim_end().to_string()).filter(|l| !l.trim().is_empty()).collect();
    if let Some(r) = &c.remove {
        let r = r.trim();
        let norm = |s: &str| s.trim().trim_start_matches(['-', '•', '*']).trim().to_lowercase();
        lines.retain(|l| norm(l) != norm(r));
    }
    if !c.add.trim().is_empty() && !lines.iter().any(|l| l.trim() == c.add.trim()) {
        lines.push(c.add.trim().to_string());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_extraction() {
        assert_eq!(extract_json(r#"{"a":1}"#).unwrap()["a"], 1);
        assert_eq!(extract_json("Here you go:\n```json\n{\"a\":2}\n```\nEnjoy").unwrap()["a"], 2);
        assert_eq!(extract_json("Sure! {\"a\":3} hope that helps").unwrap()["a"], 3);
        assert_eq!(extract_json("\"{\\\"a\\\":4}\"").unwrap()["a"], 4);
        assert!(extract_json("no json here").is_none());
        assert!(extract_json("[1,2]").is_none());
    }

    #[test]
    fn claude_output_parsing() {
        let out = "{\"type\":\"system\"}\n{\"type\":\"result\",\"is_error\":false,\"result\":\"```json\\n{\\\"items\\\":[]}\\n```\"}\n";
        assert!(parse_claude_output(out).unwrap()["items"].is_array());
        let one = "{\"type\":\"result\",\"is_error\":true,\"result\":\"Claude AI usage limit reached\"}";
        assert_eq!(parse_claude_output(one), Err(AiError::RateLimited));
        assert!(matches!(parse_claude_output("garbage"), Err(AiError::Failed(_))));
    }

    #[test]
    fn cleaning() {
        let mut items: Vec<Value> = (0..40)
            .map(|i| json!({"name": format!("food {i}"), "kcal": 100, "prot": 5, "meal": "Lunch", "grams": 100, "est": false}))
            .collect();
        items[0] = json!({"name": "x".repeat(200), "kcal": 99999, "prot": -5, "meal": "Brunch", "grams": null});
        items[1] = json!({"name": "", "kcal": 1});
        items[2] = json!("not an object");
        let (out, note) = clean_food(&json!({"items": items, "note": "ok"}), "08:00");
        assert_eq!(out.len(), 30);
        assert_eq!(out[0].name.chars().count(), 80);
        assert_eq!(out[0].kcal, 5000.0);
        assert_eq!(out[0].prot, 0.0);
        assert_eq!(out[0].meal, "Breakfast", "invalid meal → time guess");
        assert_eq!(out[0].grams, None);
        assert_eq!(out[1].name, "food 3");
        assert_eq!(note, "ok");
        let (out, _) = clean_food(&json!({"items": "nope"}), "08:00");
        assert!(out.is_empty());
        let (out, _) =
            clean_food(&json!({"items": [{"name": "Dal", "kcal": 400, "prot": 20, "fiber": 12.34, "vitC": 9999, "iron": 3}]}), "08:00");
        assert_eq!(out[0].fiber, Some(12.3));
        let m = out[0].nut.clone().unwrap();
        assert_eq!((m["vitC"], m["iron"], m.get("vitD")), (5000.0, 3.0, None));
        assert!(!m.contains_key("fiber"), "fiber is kept separately");
        let (out, _) = clean_food(&json!({"items": [{"name": "X", "kcal": 1}]}), "08:00");
        assert_eq!((out[0].fiber, out[0].nut.clone()), (None, None));
    }

    #[test]
    fn errors_combined() {
        use AiError::*;
        assert!(combine_errors(&[(Provider::Claude, RateLimited), (Provider::Gemini, RateLimited)]).contains("rate limited"));
        assert!(combine_errors(&[(Provider::Claude, RateLimited), (Provider::Gemini, NotSetUp)]).contains("rate limited right now"));
        let m = combine_errors(&[(Provider::Claude, Failed("boom".into())), (Provider::Gemini, NotSetUp)]);
        assert!(m.contains("Claude: boom") && !m.contains("Gemini"), "{m}");
        assert!(combine_errors(&[(Provider::Claude, NotSetUp), (Provider::Gemini, NotSetUp)]).starts_with("No AI is set up"));
    }

    #[test]
    fn prompt_filled() {
        let p = food_prompt(&Settings::default(), "no cheese", "2 \"eggs\"\nand toast");
        assert!(p.contains("cutting from 82kg"));
        assert!(p.contains("~1500 kcal/day, protein 90-110 g/day"));
        assert!(p.contains("chicken breast 165/31"));
        assert!(p.contains("Extra house rules he has set: no cheese."));
        assert!(p.contains(r#"He reports: "2 \"eggs\"\nand toast"."#));
    }

    #[test]
    fn rules_chat() {
        let (r, c) = clean_rules_reply(
            &json!({"reply": "Got it. I'll add that. Anything else? More.", "change": {"add": "No oil", "remove": null}}),
        );
        assert_eq!(r, "Got it. I'll add that.");
        assert_eq!(c, Some(RuleChange { add: "No oil".into(), remove: None }));
        let (_, c) = clean_rules_reply(&json!({"reply": "Ok", "change": {"add": "", "remove": null}}));
        assert!(c.is_none());
        let t = apply_rule_change("- no cheese\nno oil", &RuleChange { add: "max 2 eggs".into(), remove: Some("no cheese".into()) });
        assert_eq!(t, "no oil\nmax 2 eggs");
    }

    #[test]
    fn b64() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_decode("Zm9vYmE=").unwrap(), b"fooba");
        assert_eq!(base64_decode("data:image/png;base64,Zm8=").unwrap(), b"fo");
    }

    #[test]
    fn args_shape() {
        let a = claude_args("hi", "");
        assert_eq!(a[0..2], ["-p".to_string(), "hi".to_string()]);
        assert!(!a.contains(&"--model".to_string()));
        let a = claude_args("hi", "sonnet");
        assert_eq!(a[a.len() - 2..], ["--model".to_string(), "sonnet".to_string()]);
    }
}
