# PrivateI

A private, light Mac app for tracking food, food stock, medicines and training.
Everything stays on your Mac. AI is only used for photos, foods the built-in tables
don't know, and the house-rules chat.

## Install (Apple Silicon Mac)

1. Open the latest release: **GitHub → Releases** in this repo.
2. Download `PrivateI_<version>_aarch64.dmg`, open it, and drag **PrivateI** into **Applications**.
3. The app is ad-hoc signed (not notarised), so run this once in Terminal:

   ```bash
   xattr -dr com.apple.quarantine /Applications/PrivateI.app
   ```

4. Open PrivateI from Applications.

## First run

- **Settings (gear)** → set your weight, daily kcal target and protein floor.
- **AI** (optional): PrivateI uses your installed **Claude Code** CLI first (it looks in
  `~/.local/bin`, `~/.claude/local`, `~/.homebrew/bin`, `/opt/homebrew/bin`, `/usr/local/bin`
  and your nvm folders; set "Claude path" if yours is elsewhere). Make sure you have run
  `claude` once in Terminal and logged in. You can add a Gemini key as a fallback.
  Press **Test the AI** to see which one answers.
- **Add** → type what you ate ("200g chicken breast, 2 eggs and 1 banana") and press
  **Work out the calories**. Known foods are worked out on the Mac; only unknown ones go to the AI,
  and what the AI learns is remembered for next time.

## Bring your data over from Larder

Settings → **Restore / import…** → pick your `larder-data.json`. Both the old
`larder-export` format and the new `privatei-export` format are read. The import says how
many records it brought in. Importing again updates the same records rather than duplicating them.

## Backup

Settings → **Back up…** saves a `privatei-backup-YYYY-MM-DD.json`. It holds all your data
but never your AI keys. Keep backups out of git (the `.gitignore` already blocks
`*larder*.json` and `*privatei*.json`).

The live database is `~/Library/Application Support/app.privatei.desktop/privatei.sqlite`.

## Development

```bash
npm ci
npm run tauri dev                         # run the app
cargo test -p privatei-core --features cli
npm run test:ui                           # Playwright against the mock build
```

See [CLAUDE.md](CLAUDE.md) for the architecture and the rules this code follows.
