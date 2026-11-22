# Private-I — notes for working on this repo

The app's name is **Private-I**. Code, crates, the repo and the bundle id keep the
lowercase `privatei` (`app.privatei.desktop`, `privatei-core`, `privatei-export`) —
changing the bundle id would move the user's data folder and break updates.

Personal food, food-stock, medicine and training tracker. Mac app today (Apple Silicon),
Android (maybe iPhone) later.

## Architecture

```
crates/privatei-core/   pure Rust: data, storage, calculations, AI calls. No Tauri, no UI.
apps/desktop/           Tauri 2 app: thin #[tauri::command]s that call privatei-core
ui/                     plain HTML + CSS + vanilla JS, built with Vite
tests/fixtures/         made-up sample export (never real data)
```

- **The UI never computes anything important.** It `invoke`s a command and renders the result.
  Every number (totals, targets, burn, due/overdue doses, history, budget, stock warnings)
  comes from `privatei_core::Core` (`crates/privatei-core/src/app.rs`).
- `Core` keeps every table in memory (`store.rs`) and writes through to SQLite
  (`rusqlite`, `bundled`). Each table: `id TEXT PRIMARY KEY, updated_at INTEGER (ms),
  deleted INTEGER (tombstone), device TEXT, data TEXT (JSON)`. `Store::changes_since(ts)`
  returns every changed row including tombstones (sync-ready; no sync yet). Schema version
  is `PRAGMA user_version` (`migrate()` in `store.rs`). Corrupt rows are skipped at startup.
- `local` table (AI order/keys/models/claude path) is never synced or exported.
- AI calls never hold the `Mutex<Core>`: `prepare_*` under the lock → `ai::ask` on a blocking
  thread → `finish_*` under the lock.
- Dates are LOCAL calendar dates `YYYY-MM-DD` (`dates.rs`). Never use UTC for "today".

## Rules

- No Electron, no React or heavy UI frameworks. Keep dependencies few.
- privatei-core must build for `aarch64-apple-darwin` and `aarch64-linux-android`:
  rusqlite `bundled`, reqwest with rustls (no OpenSSL). The Claude CLI is behind feature `cli`
  (desktop only).
- Claude: run the user's installed `claude` with `std::process::Command`, args as an array,
  no shell, in a scratch dir under the app cache, 120 s timeout, model name validated
  `[A-Za-z0-9.-]{0,40}`. **Never** use an Anthropic API key or claude.ai internal endpoints.
- Gemini: key in the `x-goog-api-key` header, never in the URL.
- Clean every AI answer (`ai::clean_food`, `ai::clean_rules_reply`); the UI only ever sets
  `textContent` (see `ui/src/dom.js`; `h()` refuses `html`).
- Colours: Tokyo Night in dark mode, Tokyo Night Day in light mode (tokens at the top of
  `ui/src/styles.css`; `--energy` blue, `--protein` green, `--accent` purple).
- Layout: desktop first (sidebar + `.cols` two-column grid, centered dialogs) at ≥ 760 px;
  the phone layout (bottom tabs, bottom sheets) below that, for 360 px and the future Android app.
- Typing must never lose focus or text when data changes: screens mount their inputs once and
  only re-render read-only containers.
- Personal data never goes in git.
- Add a test for every bug fix.

## Nutrients and food databases

- `nutrients.rs` is the catalog (~40 nutrients: key, label, unit, group, daily reference,
  target/limit/info, USDA ids, Swiss column). Amounts are a map (`Nut`) keyed by catalog key;
  **a missing key means unknown, never zero**. `Log.fiber` is kept separately; `Log.nut` has the rest;
  `Log.food_ref` ("usda:167512") points back to the database entry.
- `fooddb.rs`: SQLite + FTS5. The app opens the shipped read-only `resources/foods.sqlite`
  (USDA) plus a writable `foods-user.sqlite` in the data folder for downloads (Swiss).
- `importers.rs` (feature `import`): USDA CSV and Swiss xlsx (calamine). Build the shipped DB with
  `npm run build:fooddb` (downloads USDA SR Legacy + Foundation into `target/fooddb`).
  The Swiss DB is downloaded by the app from naehrwertdaten.ch, not redistributed.
- Not allowed to ship: India IFCT 2017 (NIN copyright). Open Food Facts is ODbL (share-alike):
  keep any cache in its own file with attribution.
- The mock uses `ui/tests/fixtures/usda-sample.json` (public-domain USDA subset) and
  `catalog.json` (generated from `nutrients.rs`; regenerate if the catalog changes).

## Tests (run before every commit)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p privatei-core --features cli,import   # unit + integration (fake claude, mock HTTP)
npm run test:ui                                # Playwright, WebKit + Chromium, mockIPC
```

UI tests run against `vite build --mode mock`, which bundles `ui/tests/mock-backend.js`
(an in-memory stand-in installed with `mockIPC`). Production builds never include it.
Useful URL params for the mock build: `?seed=demo`, `?rows=3000`, `?today=YYYY-MM-DD`, `?hhmm=HH:MM`.

## Releasing

1. Bump the version in `Cargo.toml` (`[workspace.package]`), `apps/desktop/tauri.conf.json`
   and `package.json`.
2. Commit, then `git tag vX.Y.Z && git push origin main vX.Y.Z`.
3. `.github/workflows/release.yml` runs the tests, then builds on `macos-latest` with
   `tauri-apps/tauri-action@v0 --target aarch64-apple-darwin`, ad-hoc signed
   (`signingIdentity: "-"`), and publishes a GitHub Release with the `.dmg`.
   It can also be started by hand (workflow_dispatch).

Auto-update: `tauri-plugin-updater` reads
`https://github.com/sudo-panda/privatei/releases/latest/download/latest.json` (written by
tauri-action). Update bundles are signed with the key in the `TAURI_SIGNING_PRIVATE_KEY`
repo secret; the public half is `plugins.updater.pubkey` in `tauri.conf.json`. The owner's
copy of the private key is `~/.tauri/privatei.key` — if it's lost, installed apps can't
accept new updates and need one manual reinstall with a new key.

Release profile: `lto = true`, `opt-level = "s"`, `codegen-units = 1`, `strip = true`
(but not for build scripts/proc macros — stripping those dylibs breaks them on macOS).

## Android later

`cargo ndk -t arm64-v8a build -p privatei-core` already runs in CI, so the core is ready.
Two ways to ship:

**A. Tauri mobile, reusing the same UI (least work)**
1. `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`
2. Install Android Studio, SDK + NDK; set `ANDROID_HOME` and `NDK_HOME`.
3. In `apps/desktop/src/main.rs`, move `main()`'s builder into
   `#[cfg_attr(mobile, tauri::mobile_entry_point)] pub fn run()` in a `lib.rs`, add
   `[lib] crate-type = ["staticlib", "cdylib", "rlib"]`, and keep `main.rs` calling `run()`.
4. Depend on `privatei-core` **without** the `cli` feature for Android
   (`[target.'cfg(target_os = "android")'.dependencies]`); Claude is then skipped quietly
   and Gemini is used.
5. `npx tauri android init`, then `npx tauri android dev` / `npx tauri android build --apk`.
6. Replace the photo `<input type=file>` with `capture="environment"` for the camera.

**B. Native Kotlin app using privatei-core through UniFFI**
1. Add a `crates/privatei-ffi` crate (`crate-type = ["cdylib"]`) depending on `privatei-core`
   and `uniffi`; expose `Core` methods as a `#[uniffi::Object]` (views are already `Serialize`,
   so they can cross as JSON strings or as `uniffi::Record`s).
2. `cargo ndk -t arm64-v8a -t armeabi-v7a -o app/src/main/jniLibs build -p privatei-ffi --release`
3. `cargo run --bin uniffi-bindgen generate --library target/.../libprivatei_ffi.so --language kotlin --out-dir app/src/main/java`
4. Build the screens in Jetpack Compose, calling the generated bindings.

**AI on the phone:** Gemini over HTTPS (same `ai::gemini` code path, key stored in the
`local` table). Where supported (Pixel 8+/Galaxy S24+ with AICore), use **Gemini Nano**
on-device via the ML Kit GenAI / AICore API for text estimates, falling back to cloud Gemini
for photos or when Nano isn't available. Phones can't run the Claude CLI.
