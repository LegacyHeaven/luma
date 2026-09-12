# Changelog

All notable changes to Luma are documented here.

## 2.9.2

- Fixed a Windows-only build error in 2.9.1's WebView2 background fix
  below (an unwrapped call into an `unsafe` COM method) - 2.9.1 never
  actually published because of it, so this is the first release that
  carries these changes.
- A few animations were tightened up for smoother, cheaper compositing:
  the focus scan-sweep on the Default theme's search box now moves via
  `transform` instead of animating `left` (same visual sweep, no layout
  recalculation every frame), and the four themes with a continuous
  search-box glow pulse (Default, Amber, Emerald, Pink) now hint the
  browser to optimize those layers ahead of time (`will-change`). Looked
  into a fuller rework (moving each glow onto its own composited layer
  instead of animating `box-shadow`/`border-radius` directly) - a real
  option if these still feel heavy on lower-end hardware, but a bigger,
  more visually risky change than this pass.

## 2.9.1 (source only - see 2.9.2)

- Fixed the repo README (and the icon-regeneration script) pointing at a
  stale, unused placeholder image instead of the real app icon - the
  purple spyglass that's actually built into Luma. That stale file has
  been removed; the README and `npm run icons` both now use the same
  real icon everywhere.
- On Windows, the spotlight window's transparency is now also enforced
  at the WebView2 level, not just Tauri's own window compositing -
  turns off WebView2's own default background color (opaque white)
  directly on the underlying controller. Tauri's `.transparent(true)`
  and the DWM system-backdrop fix (2.2.0-era) already handle the window
  itself; this closes a separate place a solid box could still show
  through around the pill on some Windows/WebView2 Runtime
  combinations. Same fix applied to the position-picker window.

## 2.9.0

- Luma ships as a single portable executable with no installer - great for
  portability, but it meant a fresh download sitting in Downloads had no
  Start Menu entry, no desktop icon, and self-updated wherever it happened
  to be left. On Windows and Linux, the first run from anywhere other than
  its own install folder now copies itself into a stable per-user app
  folder (`%LOCALAPPDATA%\Luma` on Windows, `~/.local/share/Luma` on
  Linux), creates a desktop shortcut (and Start Menu entry on Windows),
  and relaunches from there - the same first-run handoff apps like Discord
  give you, without needing an actual installer or admin rights. Every
  step is best-effort: if anything about it fails, Luma just keeps running
  from wherever it was launched instead of refusing to start. Not changed
  on macOS yet - a raw executable doesn't get the same treatment as a real
  `.app` bundle, and that's a bigger job for another release. Once
  installed, the existing in-app updater continues working in place as
  before.

## 2.8.0

- `!open` finds a lot more now. On Windows it falls back to the same app
  catalog Windows' own Start menu search and the shell:appsfolder view
  use, so Store/UWP apps (Calculator, Settings, Photos, the UWP build of
  some apps) launch even though they never had a `.lnk` shortcut for the
  old scan to find. The manual app picker (when `!open` still can't find
  something) now also pops open that same shell:appsfolder view first, so
  it's easy to see everything installed before falling back to a plain
  file browse.
- `!open` on Linux now also checks the Snap and Flatpak desktop-file
  locations, not just the traditional `/usr/share/applications` spots -
  either kind of install was invisible to it before even though it showed
  up fine in the system's own app launcher. Also fixed `.desktop` entries
  whose `Exec=` line runs through a wrapper (Flatpak's `Exec=flatpak run
  ...`, for example) - previously only the first word of that line was
  kept, so a Flatpak app would "launch" a bare `flatpak` with no
  arguments and go nowhere.

## 2.7.0

- Reworked Settings into six tabs (General, Search & apps, Appearance,
  Spotlight window, Updates, Advanced) instead of one long scrolling page -
  nothing moved between tabs changed behavior, it's the same fields, just
  easier to find. Advanced only shows up once you've turned on debug
  logging (or found it with Shift+L, same as before).
- Added a real theme marketplace under Appearance - browse community
  themes with live color previews and install one with a click, no
  restart needed. There's also a plain "paste a theme.css URL" box for
  anything not in the official list yet. Installed themes just show up
  in the theme picker above it like any other theme.

## 2.6.0

- Redesigned four of the five themes from the ground up instead of just
  tweaking colors - Amber is now a proper retro CRT terminal (scanlines,
  a flicker overlay, square corners, mechanical step-timed motion),
  Emerald leans into an organic/botanical feel (soft blob shapes, downward
  drifting particles, italic serif-ish type), Material Blue goes fully
  flat (no glow, no particles, pure elevation shadows, nothing floats on
  hover), and Pink got a full playful pass (pill shapes everywhere,
  spring/bounce easing, confetti-colored particles). Default keeps its
  original look, with just one new touch - the search box does a single
  light sweep across itself the moment you focus it.
- Removed the blinking `_` cursor next to the search prompt in every
  theme - it only ever showed up in Default and Amber to begin with, and
  it wasn't earning its keep.

## 2.5.0

- Replaced the "More search engines" list's ~70-checkbox grid with a
  dropdown - pick one to turn it on, remove it later with the &#10005; on
  its chip, same pattern the custom search engines list already used.
  The old grid stayed printed in full no matter how few of those engines
  were actually turned on; now only the ones you've added show up at
  all, and adding one is a single click on the dropdown instead of
  hunting for its checkbox in a wall of ~70.
- Smoothed the spotlight's fade in/out - longer, gentler easing curves
  instead of the plain `ease-in`/`ease-out` browser defaults, plus a
  subtle rise/settle (a few pixels of vertical motion alongside the
  existing scale and opacity) instead of a flat scale-and-fade. The
  window-hide grace period was bumped to match the slightly longer fade
  so the window never disappears mid-animation.

## 2.4.4

- `!mypc` on Windows no longer goes through the `search-ms:` protocol at
  all. 2.4.3's bare `search-ms:query=` fallback looked safe (no error
  dialog) but live testing plus the new diagnostic log showed it wasn't
  actually searching anything - Explorer was just jumping to whichever
  folder happened to share the query's name, the same "opens some
  unrelated folder instead of searching" behavior Julian originally
  reported, just with a different folder. Combined with 2.4.2's finding
  that the documented `crumb=location:` parameter throws a "no app"
  error no matter how it's encoded, that protocol just isn't reliable
  for this on Windows 11. `!mypc` now does its own bounded,
  case-insensitive filename search of your home folder (skipping
  `AppData`, `.git`, `node_modules`, and a few other heavy/irrelevant
  folders, capped at a few seconds and 40,000 entries) and reveals the
  first match directly in Explorer - the same self-contained approach
  `!mypc` already used on macOS (`mdfind`) and Linux (`find`), instead
  of leaning on an OS URI protocol whose real-world behavior kept not
  matching its own documentation.

## 2.4.3

- Reverted the `!mypc` location-scoped search added in 2.4.1/2.4.2.
  2.4.2's fix matched Microsoft's own documented syntax for
  `search-ms:`'s `crumb=location:` parameter exactly (URL-encoded
  path and all), but live testing showed Windows still can't resolve
  it - it throws the same "Download an app to open this link" dialog
  either way. Rather than guess at another encoding blind, `!mypc`
  now sends a plain `search-ms:query=` request with no location
  crumb, which is what 2.4.0 did before this regression - it opens
  Windows Search without the "no app" error, though the search isn't
  scoped to a specific folder. It also logs the exact URI it builds
  to the in-app debug console, so the next attempt at real
  location-scoping can be checked against real data instead of
  another blind guess.

## 2.4.2

- Fixed `!mypc` throwing a "download an app to open this" dialog instead
  of searching. The 2.4.1 fix for the Documents-folder fallback added a
  location to the `search-ms:` request but sent it unencoded - Windows
  requires that value URL-encoded (`C:\Users\name` as
  `C%3A%5CUsers%5Cname`), and without that it misreads the whole thing
  as an attempt to open an unrecognized `location:` link instead of a
  search request. Confirmed correct against Microsoft's own
  documentation for the `crumb=location:` parameter this time, not just
  code review.

## 2.4.1

- Fixed the built-in browser window losing its close, minimize, and
  maximize buttons - and its whole title bar - on sites with a strict
  Content-Security-Policy (Google search results included). The custom
  title bar drew itself using inline styles the site's CSP silently
  blocked, since v2.3.0 made that title bar the window's only chrome.
  It now sets styles through the CSSOM property API instead, which
  isn't subject to that restriction.
- Fixed `!mypc` opening the Documents folder instead of an actual
  Windows Search results view - the `search-ms:` request now carries an
  explicit search location, which Windows needs to run the query instead
  of falling back to a default folder.

## 2.4.0

- Cleaned up the codebase - removed explanatory code comments throughout
  the Rust and JavaScript source and every theme's CSS. No behavior
  change; verified with a clean `cargo build`, `cargo clippy`, `cargo
  fmt --check`, and a syntax check on every JS file.

## 2.3.0

- Luma now draws its own title bar - on every platform, the main window
  and the built-in browser window no longer show the OS's own frame.
  Dragging works from the empty space in the bar, and it has its own
  minimize, maximize/restore, and close controls styled to match the
  active theme, similar to apps like Discord that draw their own window
  chrome instead of relying on the OS's.

## 2.2.1

- Fixed the auto-updater on Windows: it downloaded the new build but never
  actually installed it, because the helper process responsible for
  swapping the files in could be killed the moment Luma itself exited,
  before it finished. The swap now happens directly inside Luma itself,
  synchronously, with no separate helper process (and no PowerShell)
  involved at all - the same approach macOS and Linux already used.

## 2.2.0

- Added `!open <name>`: launches an installed app by name (Windows Start
  Menu, macOS `open -a`, Linux `.desktop` lookup). When it can't find a
  match, it opens a native file picker so you can point at the app
  directly - that choice is remembered under Settings' "Custom selected
  apps" so the same `!open <name>` launches it directly next time.
- The engine picker now shows just Google, MyPC, and Open by default
  instead of the full ~70-engine catalog. Every other built-in engine can
  be turned back on from Settings' "More search engines" list.
- Fixed the spotlight pill's glow being clipped at the window edge - the
  spotlight window now gives it enough room to render fully on every side.
- Fixed a color mismatch that showed up when scrolling the Settings page:
  the page background is now driven entirely by the active theme instead
  of partly by a fixed fallback color.
- Fixed the system tray icon rendering at a much lower resolution than
  the rest of the app on Windows.

## 2.1.1

- Fixed the app icon not being applied to the built executable.
- Fixed the version shown in Settings not matching the tagged release.
- Fixed the Settings page showing inconsistent colors once scrolled.
- Fixed `!mypc` not being registered.
- Fixed the spotlight window showing a visible box around the pill
  instead of floating transparently over the desktop.
- Fixed the built-in browser's toolbar buttons not responding to clicks,
  and made the toolbar follow the active theme.
- Hotfix: a permissions-manifest change meant to cover two new commands
  had the unintended, app-wide effect of requiring an explicit ACL grant
  for every command in Luma - breaking the whole UI. Reverted the
  unintended effect and granted every command explicitly going forward.

## 2.1.0

- Added an in-app updater: checks GitHub Releases, downloads, verifies,
  and installs a new build without leaving Luma.
- Added custom search engines, configurable from Settings.
- Added `!mypc` - hands off to the OS's own file search (Windows Search,
  Spotlight via `mdfind`, or `find` on Linux).
- Fixed a freeze affecting some window/updater interactions.
- Fixed the auto-updater silently stalling after a download completed.
- General theming and UI cleanup.

## 2.0.0

- Added a "pick position" overlay for placing the spotlight window
  anywhere on screen, saved per monitor.
- Added animations and gave each built-in theme more of its own visual
  identity.
- Redesigned the spotlight window and shrank the default main-window size.
- Added an in-app debug console (`Shift+L` in Settings) with request/error
  logging.
- Fixed the spotlight window rendering as a solid box instead of a
  floating pill.
- Fixed the Settings page's Save button being unreachable once its
  content grew taller than the window.
- Fixed the default theme not re-seeding on app updates, and a
  single-instance launch race.

## 0.1.0

First cut of Luma: a desktop search launcher (Linux, macOS, Windows) with
a `!bang` search box (BangDeck), a floating spotlight window on a global
shortcut, about 70 built-in search engines, and CSS-variable theming.
