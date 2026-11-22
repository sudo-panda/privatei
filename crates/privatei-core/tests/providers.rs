//! AI providers against a fake `claude` script and a local mock HTTP server.
#![cfg(unix)]

use privatei_core::ai::{self, AiConfig, AiRequest, Image, Provider};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("privatei-{tag}-{}", privatei_core::store::new_id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A fake `claude` that records its arguments and prints a result line.
#[cfg_attr(not(feature = "cli"), allow(dead_code))]
fn fake_claude(dir: &Path, body: &str) -> PathBuf {
    let p = dir.join("claude");
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn cfg(claude: Option<PathBuf>, gemini_base: &str, key: &str, order: Vec<Provider>) -> AiConfig {
    AiConfig {
        order,
        gemini_key: key.into(),
        gemini_model: "gemini-flash-latest".into(),
        gemini_base: gemini_base.into(),
        claude_model: String::new(),
        claude_path: Some(claude.unwrap_or_else(|| PathBuf::from("/nonexistent/claude"))),
        scratch_root: tmpdir("scratch"),
        timeout: Duration::from_secs(20),
    }
}

struct Captured {
    request_line: String,
    headers: Vec<String>,
    body: String,
}

/// Serve one HTTP response; send back what the client sent.
fn mock_server(status: u16, body: &'static str) -> (String, mpsc::Receiver<Captured>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut request_line = String::new();
        r.read_line(&mut request_line).unwrap();
        let mut headers = vec![];
        let mut len = 0usize;
        loop {
            let mut h = String::new();
            r.read_line(&mut h).unwrap();
            let h = h.trim_end().to_string();
            if h.is_empty() {
                break;
            }
            if let Some(v) = h.to_lowercase().strip_prefix("content-length:") {
                len = v.trim().parse().unwrap();
            }
            headers.push(h);
        }
        let mut b = vec![0; len];
        r.read_exact(&mut b).unwrap();
        let resp = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        s.write_all(resp.as_bytes()).unwrap();
        tx.send(Captured { request_line, headers, body: String::from_utf8(b).unwrap() }).unwrap();
    });
    (format!("http://{addr}/v1beta"), rx)
}

const GEMINI_OK: &str = r#"{"candidates":[{"content":{"parts":[{"text":"{\"items\":[{\"name\":\"Mystery curry\",\"grams\":300,\"kcal\":450,\"prot\":20,\"est\":true,\"meal\":\"Dinner\"}],\"note\":\"\"}"}]}}]}"#;

#[cfg(feature = "cli")]
#[test]
fn prompt_reaches_claude_as_one_untouched_argument() {
    let dir = tmpdir("args");
    let out = dir.join("arg2");
    let count = dir.join("count");
    let cwd = dir.join("cwd");
    let script = format!(
        "printf '%s' \"$2\" > '{}'\nprintf '%s' \"$#\" > '{}'\npwd > '{}'\nprintf '%s\\n' '{{\"type\":\"system\"}}'\nprintf '%s\\n' '{{\"type\":\"result\",\"is_error\":false,\"result\":\"Here:\\n```json\\n{{\\\"ok\\\":true}}\\n```\"}}'",
        out.display(),
        count.display(),
        cwd.display()
    );
    let bin = fake_claude(&dir, &script);
    let prompt = "He said \"hi\" and 'bye' $(touch /tmp/privatei-pwned) `id` ; rm -rf / \nsecond line\n\ttab $HOME";
    let c = cfg(Some(bin), "http://127.0.0.1:9", "", vec![Provider::Claude]);
    let ans = ai::ask(&c, &AiRequest { prompt: prompt.into(), image: None }).unwrap();
    assert_eq!(ans.provider, Provider::Claude);
    assert_eq!(ans.value["ok"], true);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), prompt, "prompt untouched");
    assert_eq!(std::fs::read_to_string(&count).unwrap(), ai::claude_args(prompt, "").len().to_string());
    let ran_in = std::fs::read_to_string(&cwd).unwrap();
    assert!(ran_in.trim().starts_with(std::fs::canonicalize(&c.scratch_root).unwrap().to_str().unwrap()), "{ran_in}");
    assert!(!Path::new("/tmp/privatei-pwned").exists());
    // The scratch folder is cleaned up.
    assert_eq!(std::fs::read_dir(&c.scratch_root).unwrap().count(), 0);
}

#[cfg(feature = "cli")]
#[test]
fn claude_reads_the_photo_file() {
    let dir = tmpdir("photo");
    let seen = dir.join("seen");
    let script = format!(
        "ls > '{}'\ncat meal.png >> '{}'\nprintf '%s\\n' '{{\"type\":\"result\",\"is_error\":false,\"result\":\"{{\\\"items\\\":[]}}\"}}'",
        seen.display(),
        seen.display()
    );
    let bin = fake_claude(&dir, &script);
    let c = cfg(Some(bin), "http://127.0.0.1:9", "", vec![Provider::Claude]);
    let req = AiRequest { prompt: "what is this".into(), image: Some(Image { bytes: b"PNGDATA".to_vec(), ext: "png".into() }) };
    ai::ask(&c, &req).unwrap();
    let s = std::fs::read_to_string(&seen).unwrap();
    assert!(s.contains("meal.png") && s.contains("PNGDATA"), "{s}");
}

#[cfg(feature = "cli")]
#[test]
fn claude_times_out() {
    let dir = tmpdir("slow");
    let bin = fake_claude(&dir, "sleep 10");
    let mut c = cfg(Some(bin), "http://127.0.0.1:9", "", vec![Provider::Claude]);
    c.timeout = Duration::from_millis(500);
    let t = std::time::Instant::now();
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert!(e.contains("longer than"), "{e}");
    assert!(t.elapsed() < Duration::from_secs(5));
}

#[cfg(feature = "cli")]
#[test]
fn rejects_bad_model_names() {
    let dir = tmpdir("model");
    let bin = fake_claude(&dir, "echo should-not-run");
    let mut c = cfg(Some(bin), "http://127.0.0.1:9", "", vec![Provider::Claude]);
    c.claude_model = "opus; rm -rf /".into();
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert!(e.contains("not valid"), "{e}");
}

#[test]
fn gemini_key_in_header_never_url() {
    let (base, rx) = mock_server(200, GEMINI_OK);
    let c = cfg(None, &base, "KEY-123", vec![Provider::Claude, Provider::Gemini]);
    let req = AiRequest { prompt: "curry".into(), image: Some(Image { bytes: vec![1, 2, 3], ext: "jpg".into() }) };
    let ans = ai::ask(&c, &req).unwrap();
    assert_eq!(ans.provider, Provider::Gemini, "missing claude skipped quietly");
    assert_eq!(ans.value["items"][0]["name"], "Mystery curry");
    let got = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(got.request_line.starts_with("POST /v1beta/models/gemini-flash-latest:generateContent "), "{}", got.request_line);
    assert!(!got.request_line.contains("KEY-123"));
    assert!(got.headers.iter().any(|h| h.to_lowercase() == "x-goog-api-key: key-123"));
    let body: serde_json::Value = serde_json::from_str(&got.body).unwrap();
    assert_eq!(body["generationConfig"]["responseMimeType"], "application/json");
    assert_eq!(body["contents"][0]["parts"][1]["inline_data"]["data"], "AQID");
    assert_eq!(body["contents"][0]["parts"][1]["inline_data"]["mime_type"], "image/jpeg");
}

#[test]
fn gemini_errors() {
    let (base, _rx) = mock_server(400, "{}");
    let c = cfg(None, &base, "k", vec![Provider::Gemini]);
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert!(e.contains("check the Gemini key or model"), "{e}");

    let (base, _rx) = mock_server(429, "{}");
    let c = cfg(None, &base, "k", vec![Provider::Gemini]);
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert!(e.contains("rate limited right now"), "{e}");

    let c = cfg(None, "http://127.0.0.1:9", "", vec![Provider::Gemini, Provider::Claude]);
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert!(e.starts_with("No AI is set up"), "{e}");
}

#[cfg(feature = "cli")]
#[test]
fn every_provider_rate_limited() {
    let dir = tmpdir("rl");
    let bin = fake_claude(
        &dir,
        "printf '%s\\n' '{\"type\":\"result\",\"is_error\":true,\"result\":\"Claude AI usage limit reached|1700000000\"}'",
    );
    let (base, _rx) = mock_server(429, "{}");
    let c = cfg(Some(bin), &base, "k", vec![Provider::Claude, Provider::Gemini]);
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert_eq!(e, "The AI is rate limited right now. Try again in a minute.");
}

#[cfg(feature = "cli")]
#[test]
fn falls_back_to_gemini_and_combines() {
    let dir = tmpdir("fb");
    let bin = fake_claude(&dir, "printf '%s\\n' 'not json at all'; exit 1");
    let (base, _rx) = mock_server(200, GEMINI_OK);
    let c = cfg(Some(bin.clone()), &base, "k", vec![Provider::Claude, Provider::Gemini]);
    assert_eq!(ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap().provider, Provider::Gemini);

    let (base, _rx) = mock_server(500, "{}");
    let c = cfg(Some(bin), &base, "k", vec![Provider::Claude, Provider::Gemini]);
    let e = ai::ask(&c, &AiRequest { prompt: "x".into(), image: None }).unwrap_err();
    assert!(e.contains("Claude:") && e.contains("Gemini:"), "{e}");
}

#[cfg(feature = "cli")]
#[test]
fn core_estimate_local_first_then_ai_and_learns() {
    use privatei_core::app::{AiSettingsInput, Clock};
    let dir = tmpdir("core");
    let args = dir.join("prompt");
    let script = format!(
        "printf '%s' \"$2\" > '{}'\nprintf '%s\\n' '{{\"type\":\"result\",\"is_error\":false,\"result\":\"{{\\\"items\\\":[{{\\\"name\\\":\\\"Haferflocken\\\",\\\"grams\\\":40,\\\"kcal\\\":150,\\\"prot\\\":5.2,\\\"est\\\":false,\\\"meal\\\":\\\"Brunch\\\"}}],\\\"note\\\":\\\"Oats.\\\"}}\"}}'",
        args.display()
    );
    let bin = fake_claude(&dir, &script);
    let mut core = privatei_core::Core::in_memory();
    core.set_clock(Clock::Fixed { today: "2026-11-22".into(), hhmm: "08:00".into() });
    core.save_ai_settings(AiSettingsInput {
        order: vec!["claude".into()],
        gemini_key: String::new(),
        clear_gemini_key: false,
        gemini_model: String::new(),
        claude_model: String::new(),
        claude_path: bin.display().to_string(),
    })
    .unwrap();
    let p = core.prepare_estimate("150 g Skyr mit 40g Haferflocken").unwrap();
    assert_eq!(p.local.items.len(), 1);
    let (cfg, req) = p.ai.clone().unwrap();
    let ans = ai::ask(&cfg, &req);
    let v = core.finish_estimate(p, Some(ans));
    assert_eq!(v.items.len(), 2);
    assert_eq!(v.items[1].meal, "Breakfast", "invalid slot replaced by the time guess");
    assert_eq!(v.provider_text.as_deref(), Some("Claude worked it out"));
    assert!(v.unknown.is_empty());
    let sent = std::fs::read_to_string(&args).unwrap();
    assert!(sent.contains("He reports: \"40g Haferflocken\"."), "only the unknown part goes to the AI");
    // Learned: next time it's local.
    let f = core.learned_foods();
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].kcal100, f[0].prot100, f[0].piece), (375.0, 13.0, 0.0));
    let p = core.prepare_estimate("80g Haferflocken").unwrap();
    assert!(p.ai.is_none());
    assert_eq!(p.local.items[0].kcal, 300.0);
}

#[test]
fn core_estimate_without_ai_names_the_rest() {
    let mut core = privatei_core::Core::in_memory();
    core.save_ai_settings(privatei_core::app::AiSettingsInput {
        order: vec!["claude".into(), "gemini".into()],
        gemini_key: String::new(),
        clear_gemini_key: false,
        gemini_model: String::new(),
        claude_model: String::new(),
        claude_path: "/nonexistent/claude".into(),
    })
    .unwrap();
    let p = core.prepare_estimate("2 eggs, some mystery stew").unwrap();
    let (cfg, req) = p.ai.clone().unwrap();
    let v = core.finish_estimate(p, Some(ai::ask(&cfg, &req)));
    assert_eq!(v.items.len(), 1);
    assert_eq!(v.unknown, vec!["some mystery stew"]);
    assert!(v.error.unwrap().starts_with("No AI is set up"));
}
