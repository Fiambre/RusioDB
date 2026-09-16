---
name: rusiodb-dev
description: Development guide for RusioDB, a Rust/Iced desktop multi-database admin tool (SQLite/PostgreSQL/MySQL/MongoDB) at C:\Desarrollo\RusioDB. Load this whenever asked to add a feature, fix a bug, run/build/test the app, screenshot it for visual verification, or touch any file under src/, Cargo.toml, README.md, or PROJECT_CONTEXT.md in this project — even if the request doesn't mention RusioDB by name (e.g. "agrega soporte para X", "corré la app", "hay un bug en el árbol de conexiones"). Captures the driver architecture, the mandatory fmt/test/clippy/build verification sequence, the screenshot-based visual verification technique (this is a GUI app with no test-friendly headless mode), and compile-time traps that are expensive to rediscover cold — most importantly a Cargo.toml feature-flag trap in the MongoDB dependency that produces ~1300 unrelated-looking compile errors if reintroduced.
---

# RusioDB development guide

RusioDB is a Rust desktop database admin tool built with the Iced GUI
framework, styled after Navicat/QoreDB. It's a solo working project at
`C:\Desarrollo\RusioDB`, **not a git repository** — don't propose git
workflows (branches, commits, PRs) unless the user asks to initialize one
first.

The user communicates in Spanish. Project docs (`README.md`,
`PROJECT_CONTEXT.md`) and all UI strings are in Spanish — match that
language when editing them or adding new UI text.

Read `PROJECT_CONTEXT.md` at the start of any nontrivial task — it's the
living architecture doc and is kept in sync with every feature. This skill
summarizes it plus the operational traps that don't belong in a
user-facing doc.

## Architecture at a glance

- **GUI**: Iced 0.13.1, Elm-architecture (`Message` / `update` / `view`),
  using `iced::daemon` (multi-window) rather than `iced::application`
  (single-window) — needed for a custom frameless title bar on the main
  window plus a separate connection-form window.
- **Frameless title bar**: `window::Settings { decorations: false, .. }`,
  manual `window::drag(id)` / `minimize` / `maximize` / `toggle_maximize`,
  and hand-rolled double-click detection — iced has no native double-click
  event, so don't go looking for one.
- **Menus**: `iced_aw` 0.12.2 `MenuBar`, custom-styled (`menu_style()` in
  `src/app.rs`) to match the app's palette instead of `iced_aw`'s default
  gray.
- **Icons**: `iced_fonts` 0.1.1 (Bootstrap icon set) and `assets/cat.svg`
  (the app logo — a cat silhouette, rendered via `iced::widget::svg` with
  the `svg` feature on the `iced` crate).
- **Four database engines behind one `Database` enum** in `src/drivers.rs`:
  SQLite (`rusqlite`, bundled — compiles with the app, no system
  dependency), PostgreSQL (`postgres` + `postgres-native-tls` +
  `native-tls`), MySQL (`mysql` crate v26, `native-tls`), MongoDB
  (`mongodb` crate v3.x, blocking API via the `sync` feature). All four
  are driven synchronously through `tokio::task::spawn_blocking`, which is
  exactly why Mongo uses `mongodb::sync::{Client, Database, Collection,
  Cursor}` instead of its native async API — it needed to fit the same
  pattern as the other three, not introduce a second concurrency model.
- **Connections tree**: Navicat/QoreDB-style — connection → (schema level,
  for Postgres and Mongo only) → kind folders (Tablas / Colecciones /
  Vistas / Vistas materializadas / Funciones / Procedimientos) → objects.
  Empty kind-folders are skipped automatically; adding a new `ObjectKind`
  that a given driver never emits requires zero rendering changes.
- **Passwords**: never written to `connections.json`, ever. If the user
  opts in (per-connection "Guardar contraseña" checkbox, on by default),
  the password goes to the OS credential store via the `keyring` crate v4
  (`Entry::new("RusioDB", &connection_id.to_string())`). Any change that
  touches connection persistence must preserve this — it was an explicit,
  deliberate security decision, not an oversight to "simplify" away.
- **Concurrency model**: several connections can be open at once, each
  with its own `ConnState` (`Disconnected` / `Connecting` / `Connected` /
  `Error`), and each query tab has its own `running` flag. There is no
  global "busy" flag anywhere — don't reintroduce one. A query running on
  one connection/tab must never block another.
- **Custom theming** (`src/theme.rs`): `Theme::custom(name, Palette)` for
  dark/light, plus hand-picked `panel()` / `alt_row()` / `border()` /
  `accent()` / `accent_strong()` color functions. **Do not** reach for
  iced's auto-derived `extended_palette().background.weak/strong` for
  panel/row backgrounds — it blends toward the text color in linear RGB
  space, which looks washed-out gray on a dark theme with bright text.
  This was found and fixed once already; the hand-picked functions in
  `theme.rs` are the deliberate replacement.

## Key files

| File | What's in it |
| --- | --- |
| `src/app.rs` (~2000+ lines) | `App` state, connections tree rendering, menus, toolbar, connection form, query tabs, keyboard shortcuts, async tasks. Tests live in `#[cfg(test)] mod tests` at the bottom. |
| `src/drivers.rs` | `Driver` enum, `Config`, the `Database` enum with `connect`/`catalog`/`execute` per engine, `ObjectKind`, `CatalogObject::preview`, and the MongoDB-specific helpers (`parse_mongo_filter`, `documents_to_result`, `bson_cell`, `percent_encode_userinfo`). Tests include 3 `#[ignore]`d remote-integration tests. |
| `src/db.rs` | SQLite-specific execution/catalog/value-to-text conversion, `ROW_LIMIT = 500` (shared by all four drivers), the engine-agnostic `QueryResult` struct. |
| `src/connections.rs` | `ConnectionProfile` (deliberately has no password field), JSON persistence (`dirs::config_dir()/RusioDB/connections.json`), and the keyring `save_password`/`load_password`/`delete_password` functions. |
| `src/theme.rs` | Palettes and hand-picked colors — see theming note above. |
| `src/updater.rs` | Windows autoupdate: `check_for_update`/`download_update`/`apply_update` against the GitHub Releases API, plus the pure `parse_release_json` (version compare, asset lookup) that's tested without network. |
| `assets/cat.svg` / `assets/icon.ico` | App logo (SVG) and its rasterized Windows icon (generated once via `examples/gen_icon.rs`, embedded into the `.exe` by `build.rs` + `winresource`). |
| `Cargo.toml` | See the MongoDB gotcha below before touching this file. |
| `installer/rusiodb.iss` | Inno Setup script for the Windows installer (per-user install, no admin/UAC — required so the autoupdater can swap the `.exe` without elevation). |
| `.github/workflows/release.yml` | Builds and publishes a GitHub Release (raw `.exe` + installer) whenever a `vX.Y.Z` tag is pushed. |
| `README.md` / `PROJECT_CONTEXT.md` | User-facing docs and architecture notes, in Spanish, updated alongside every feature. |

## Gotchas that will cost you real time if you rediscover them cold

### The MongoDB `Cargo.toml` trap

The `mongodb` dependency must stay exactly:

```toml
mongodb = { version = "3", features = ["sync"] }
```

**Do not add `default-features = false`**, even if it seems like the
"minimal" or "explicit" thing to do (e.g. to hand-pick `rustls-tls`
alongside `sync`). Turning off default features drops the crate's
`compat-3-0-0` feature, which is what silently pulls in `bson-2` (the
crate offers `bson-2` vs `bson-3` as alternate BSON implementations, and
without either selected, internal `mongodb` crate code that assumes one of
them — e.g. `CollRef` methods — stops compiling). The failure mode is
**~1300 compiler errors that look like they're coming from inside the
`mongodb`/`bson` crates themselves**, with no obvious link back to a
`Cargo.toml` feature flag. If you ever see a wall of unrelated-looking
errors after touching MongoDB dependencies, check this first before
assuming something is broken in the driver code.

### Visual verification: this is a GUI app, so testing "it works" means looking at it

`cargo test` cannot verify that the UI actually renders correctly — there
is no headless mode. The established pattern for confirming a UI change
visually (used repeatedly and reliably across this project's history):

1. **Seed fake state, don't automate clicks.** Add a block gated by
   `if std::env::var("RUSIODB_DEMO").is_ok() { ... }` inside `App::load()`
   in `src/app.rs`, to construct fake `ConnectionEntry`/`ConnState::Connected`
   data directly (see git-free history: this has been done for Postgres,
   dark-mode, and MongoDB tree verification already — the pattern is
   proven). If you need the tree pre-expanded so a screenshot shows nested
   folders without simulating a click, populate `app.expanded:
   HashSet<String>` directly too — keys are `"{connection_id}"`,
   `"{connection_id}/{schema}"`, and
   `"{connection_id}/{schema}/{ObjectKind:?}"` (the `{:?}` is Rust's Debug
   format for the enum variant, e.g. `Collection`, `View`).
   **Why not simulate clicks instead?** Click automation via `PostMessage`
   was tried repeatedly in this project and proved unreliable — sometimes
   it worked, sometimes the target window was stale or occluded and the
   click silently landed nowhere. Seeding state directly sidesteps that
   entirely and is far more deterministic.
2. Build and launch with the env var set, backgrounded so you can keep
   working:
   ```sh
   cargo build
   (RUSIODB_DEMO=1 ./target/debug/rusiodb.exe > /tmp/rusiodb.log 2>&1 &)
   sleep 3
   ```
3. **Find the window** via PowerShell `EnumWindows` + `GetWindowText`
   matching `"RusioDB"` to get its HWND (see script sketch below).
4. **Screenshot with `PrintWindow`, flag `2` (`PW_RENDERFULLCONTENT`), not
   `CopyFromScreen`.** `CopyFromScreen` grabs whatever is topmost on the
   screen at that location — if another window (even the terminal) is
   occluding RusioDB, you silently screenshot the wrong thing.
   `PrintWindow` renders the specific HWND's content regardless of Z-order
   or occlusion, which is what actually matters here.
5. **Run the PowerShell as an inline `-Command`/here-string, not a saved
   `.ps1` file.** On this machine, loading a `.ps1` file via `-File` is
   blocked by execution policy (`UnauthorizedAccess`, "no está firmado
   digitalmente") even with `-ExecutionPolicy Bypass` passed explicitly —
   it appears to be enforced above the process level. The exact same C#
   P/Invoke code run as an inline command block works fine. Don't burn
   time debugging execution-policy flags; just inline the script.
6. Read the resulting PNG with the `Read` tool to actually look at it —
   don't just trust that the build succeeded.
7. **Always remove the temporary `RUSIODB_DEMO` block afterward** and
   re-run the full verification sequence below to confirm the codebase is
   clean again. This is throwaway scaffolding for one verification pass,
   never a permanent feature flag.
8. Clean up: `taskkill //F //IM rusiodb.exe` to kill the demo instance, and
   delete any temp screenshot files you wrote to the scratchpad.

Minimal PowerShell sketch for steps 3-4 (adapt the window-title match and
output path; run as one inline command, not a `.ps1`):

```powershell
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WinFinder {
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
}
"@
$results = New-Object System.Collections.ArrayList
$callback = {
    param($hwnd, $lparam)
    if ([WinFinder]::IsWindowVisible($hwnd)) {
        $sb = New-Object System.Text.StringBuilder 256
        [WinFinder]::GetWindowText($hwnd, $sb, 256) | Out-Null
        if ($sb.ToString() -match "RusioDB") { $results.Add("$hwnd|$($sb.ToString())") | Out-Null }
    }
    return $true
}
[WinFinder]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
$results
```

```powershell
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class ShotWin {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@
$hwnd = [IntPtr]<HANDLE_FROM_STEP_3>
$rect = New-Object ShotWin+RECT
[ShotWin]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[ShotWin]::PrintWindow($hwnd, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc)
$bmp.Save("<OUT_PATH>.png", [System.Drawing.Imaging.ImageFormat]::Png)
```

### Test purity: `App::default()` must never touch disk or the real keyring

`App::default()` (used throughout the test suite) sets `connections_path:
None` and `keyring_enabled: false`, which turns `persist_connections` and
the `sync_saved_password`/`connections::load_password`/`delete_password`
calls into no-ops. Only `App::load()` (the real entry point, called from
`main.rs`) enables both. This is what keeps `cargo test` from writing to
the developer's actual `connections.json` or OS credential store, and it's
verified by the test `default_app_never_touches_the_real_os_keyring`. If
you add new state that touches disk or the keyring, gate it the same way
— check whether the new code path is reachable from `App::default()` and
make sure it isn't, rather than adding an ad-hoc try/catch around it.

## Verification workflow — run this after every change, no exceptions

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
```

This exact sequence is documented in `README.md` and has been the
closing step of every feature added to this project. Treat a feature as
unfinished until all four pass clean.

Three integration tests are `#[ignore]`d by default (`postgres_integration`,
`mysql_integration`, `mongodb_integration`) because they need a live
server. They read connection details from environment variables:
`RUSIODB_PG_*`, `RUSIODB_MYSQL_*`, `RUSIODB_MONGO_*` — each needs `HOST`,
`PORT` (defaults to the driver's standard port), `DATABASE`, `USER`,
`PASSWORD`, `TLS` (only disabled by literal `"false"`); Mongo additionally
needs `RUSIODB_MONGO_COLLECTION` naming a real collection. Run one with:

```sh
cargo test mongodb_integration -- --ignored
```

Don't run these unless the user has a real server configured and asks for
it — they're not part of the standard verification loop.

## Working conventions in this project

- **Add new enum variants before touching call sites, and let the
  compiler enumerate the rest.** When MongoDB was added as the fourth
  driver, `Driver::Mongo` was added to the enum *first*, then `cargo
  build`'s exhaustiveness checking on every `match` over `Driver`/
  `Database` was used to find every site that needed a decision. This is
  more reliable than grepping for driver names by hand, and it's the
  expected approach for a fifth driver or a new `ObjectKind` variant too.
- **Plan nontrivial features before implementing.** The user prefers
  plan-mode for anything that touches multiple files or changes the
  window/UI model (a new driver, a new window, a new persisted concept).
  Small, contained fixes don't need it.
- **Iterate in small, visually-confirmed steps** rather than large
  batches of unverified changes — this matches how every feature in this
  project's history was actually built and reviewed.
- **This project targets Windows, macOS, and Linux**, but Windows is the
  only platform actually exercised in day-to-day development. Don't claim
  macOS/Linux behavior has been verified unless it actually has — treat it
  as a known gap (`PROJECT_CONTEXT.md`'s "Riesgos y límites conocidos"
  section tracks this explicitly).
