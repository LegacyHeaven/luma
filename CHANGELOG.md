# Changelog

All notable changes to Luma are documented here.

## 2.9.14

- Actually fixed the stray focus-border on the position-picker overlay.
  2.9.13 turned it off with the right API (`DWMWA_BORDER_COLOR` /
  `DWMWA_COLOR_NONE`) but called it too late for that particular window:
  unlike the spotlight pill, the picker was left visible immediately on
  creation, so by the time the fix ran, Windows had already started
  drawing the border and never retroactively erased it. The picker window
  is now built hidden, has the border (and backdrop/webview-background)
  fixes applied while still hidden, and is only shown afterwards - the
  same order the spotlight window already used successfully. The
  spotlight pill itself was never affected by this - its border fix really
  did land in 2.9.13.

## 2.9.13

- Replaced Esc-to-cancel on the position-picker overlay ("Pick
  position...") with a plain 10-second auto-cancel countdown, shown live
  right in its own hint text. Esc depended on this fullscreen overlay
  actually holding OS keyboard focus at the moment it was pressed, and
  that proved flaky enough on Windows across several attempts at fixing it
  outright (most recently root-caused via diagnostic logging: the window
  already held OS *foreground* the whole time - also visible as Windows
  11's own accent-colored focus border tracking it, see the next item -
  it was specifically the separate, never-actually-called `SetFocus` API
  for *keyboard* focus that was missing) that it wasn't worth keeping
  around at all. A countdown needs no keyboard focus whatsoever, so it
  can't be defeated by whatever's going on with focus on any given
  machine - cancelling the picker now only ever depends on a timer and a
  click, neither of which cares about focus.
- Fixed a stray colored line running along the very edges of the screen
  while the position picker (and, less noticeably, the spotlight pill) is
  open. That was Windows 11's own accent-colored focus border, which it
  draws around whatever window currently has keyboard focus - normally a
  subtle outline right at a window's own frame, but these windows are
  fullscreen, chromeless and fully transparent, so it showed up instead as
  a rectangle traced right along the screen's actual edges. Turned off for
  both windows.

## 2.9.12

- No user-facing change. 2.9.11's `AttachThreadInput`-based fix for the
  position-picker's Esc-to-cancel still didn't work in live re-testing,
  despite matching the standard Win32 pattern for this exact problem - so
  rather than shipping a third blind guess, this adds temporary diagnostic
  logging around the actual `SetForegroundWindow` call and its result,
  visible in the debug console, to find out what's really happening on
  this machine instead of guessing from what should happen in theory.

## 2.9.11

- Pressing Esc to cancel the position-picker overlay (Settings → Spotlight
  window → "Pick position...") now actually works reliably. 2.9.9 and
  2.9.10 both believed this was fixed by re-asserting window focus shortly
  after the picker opened (once immediately, then retried twice from a
  background thread), and live testing at the time seemed to confirm it -
  but further testing after 2.9.10 shipped found Esc still doing nothing,
  no matter how long you waited before pressing it. Waiting longer never
  had a chance of helping: Windows enforces a "foreground lock" that
  generally blocks a process from stealing keyboard focus away from
  whatever the user was last interacting with, and retrying the exact same
  restricted call later just hits the same wall again. The picker now uses
  the standard workaround for this (`AttachThreadInput`, which briefly
  shares input state with whatever currently holds focus so the OS allows
  the handoff) instead of hoping a delay would eventually get lucky.

## 2.9.10

- `!mypc` actually opens Windows Search now. 2.9.9's rewrite got the right
  idea (hand off to the real indexed search via the `search-ms:` URI) but
  the wrong mechanism: it spawned `explorer.exe <uri>` as a plain child
  process, and live testing that after shipping it found this popped
  Windows' "Open With" chooser instead of a search-results window. A bare
  `CreateProcess` (all `std::process::Command` ever does) never goes
  through the shell's own protocol-handler resolution - only
  `ShellExecute` does that, by looking `search-ms:` up in the registry the
  same way a Run dialog or a shell link would. `!mypc` now calls
  `ShellExecuteW` directly instead of hoping `explorer.exe`'s own argv
  parsing would special-case the string, which is what actually opens the
  real Windows Search index against the query.
- On Windows, Luma's config (config.toml, custom themes, everything else)
  now lives right beside the installed `luma.exe` instead of tucked away
  in `%AppData%\Roaming` - so it's there to find just by browsing to
  wherever Luma is installed, no digging through a hidden folder required.
  Falls back to the old per-user location if that folder ever turns out
  not to be writable (e.g. Luma installed somewhere needing admin rights).
  Anyone upgrading from an older version has their existing config.toml
  and themes folder copied over automatically the first time this version
  runs, so nothing already configured is lost.
- The main window's Settings entry point is now a gear icon sitting flush
  in the top-left corner, in the same 44px titlebar band as the
  minimize/maximize/close buttons - mirroring their position and hover
  behavior instead of the bordered "</LUMA>" text pill it used to be,
  which didn't read as a settings control at a glance (its only hint was a
  hover tooltip). Settings' own back-to-search link is unchanged.

## 2.9.9

- Actually fixed Escape-to-cancel on the position-picker overlay (2.9.8's
  global-hotkey fallback wasn't the real fix - live re-testing after
  shipping it found the hotkey registration itself silently failing,
  because another already-running app on the test machine had already
  claimed a bare Escape as *its own* global hotkey, and global hotkeys are
  exclusive system-wide, one owner at a time). The actual root cause was
  the picker window's `set_focus()` call, made right after it's built,
  losing a race it doesn't always win - the window isn't necessarily fully
  realized at the OS level the instant `build()` returns, so whatever had
  focus a moment earlier (typically Settings, on the "open the picker from
  Settings" path) can end up keeping it, leaving the picker's own in-page
  Escape handler never receiving the keypress. Focus is now re-asserted
  twice more, shortly after opening, which reliably wins the race the
  first call sometimes loses; the global-hotkey fallback from 2.9.8 stays
  in place as a bonus for machines where it's free to register, it just
  isn't relied on as the only fix anymore.
- Fixed the main window (and Settings) not being draggable at all, and
  double-clicking the titlebar not maximizing it either. Both go through
  Tauri's own permission system just like every other command Luma calls -
  and both permissions (`core:window:allow-start-dragging` and
  `allow-internal-toggle-maximize`) were simply missing from the app's
  capabilities file, so every drag attempt and every double-click on the
  custom titlebar was silently rejected with no error anywhere. Both are
  granted now.
- Reworked `!mypc` to actually search using Windows' own indexed search -
  the same index behind the Start Menu's and Explorer's own search boxes -
  instead of walking the filesystem by hand. The old approach could only
  ever match filenames (never file content, which the real index already
  covers), ran synchronously on Luma's own main thread, and had to give up
  after a fixed 3-second/40,000-entries budget - which a dev machine's home
  folder (git clones, `node_modules`, build output, ...) blows through long
  before ever reaching whatever was actually typed, which is exactly why it
  kept coming back empty. `!mypc` now opens a normal, live-updating
  Explorer search-results window against the real index, scoped to your
  home folder - instant, and finds everything Windows' own search would.

## 2.9.8

- Fixed Escape-to-cancel on the "Pick position" spotlight-placement overlay
  sometimes silently doing nothing, found while re-testing 2.9.6's fix for
  the same overlay live: clicking to place a spot always worked, but
  pressing Escape occasionally didn't, especially the second or later time
  the picker was opened in the same session. Root cause: cancelling on
  Escape depends on that window actually holding OS keyboard focus, and on
  Windows a newly-created always-on-top, decorationless, transparent window
  asking for focus right after it's built doesn't always win it - so the
  keypress could go to whichever window was focused a moment earlier
  instead, leaving the picker open with no visible way out (short of
  clicking somewhere, which places a spot instead of cancelling, or
  Alt-Tabbing away, which happens to cancel it too since losing focus
  already cancels the picker). Escape now also works as a real OS-level
  hotkey - registered only while the picker is open, unregistered the
  instant it closes - so it fires no matter which window currently has
  focus, on top of (not instead of) the picker's own in-page Escape
  handling.

## 2.9.7

- On Windows, `!open` (and the very first Discord-style relaunch that
  creates your desktop/Start Menu shortcut) could flash a plain console
  window on screen for a split second. Luma itself has no console of its
  own, so whenever it had to hand off through `cmd.exe`, `powershell.exe`
  (part of `!open`'s Store/UWP-app fallback), or `cscript.exe` (the
  one-time shortcut-creation step), Windows allocated a brand new one for
  that helper process - visible for exactly as long as the helper took to
  do its job and exit. Every one of those spawns now runs with
  `CREATE_NO_WINDOW`, so no window ever appears at all.

## 2.9.6

- Actually fixed the "Pick position" overlay being permanently stuck
  (2.9.5 changed something real but not the actual cause - clicking and
  Esc still did nothing after upgrading, confirmed by testing it live).
  The real root cause: the position-picker window was simply never
  listed in `src-tauri/capabilities/default.json`'s `windows` array, so
  Tauri's own permission system silently rejected every single command
  call this window ever made - `report_spotlight_position` on click,
  `cancel_position_pick` on Esc, all of it, every time, with no error
  surfaced anywhere (a plain JS promise rejection swallowed by an empty
  `.catch()`). Mouse tracking (the crosshair) kept working throughout
  because that's pure DOM/CSS with no backend call involved, which is
  exactly what made this look like a timing race instead of a permissions
  gap. Added `"position-picker"` to that window list; the picker now
  places and cancels correctly.

## 2.9.5

- Fixed the "Pick position" spotlight-placement overlay (Settings ->
  Spotlight window -> "Choose position on screen") occasionally becoming
  completely stuck: clicking to place it and pressing Esc to cancel would
  both silently do nothing, trapping you behind a fullscreen overlay with
  no way out short of force-closing the app. Root cause: its click/Escape
  handlers only got wired up after a one-time, 2-second poll for the
  in-app bridge to be ready gave up - and it gave up for good, not just
  that one time, so if the bridge (plausibly delayed by this window's
  extra Windows-specific backdrop/transparency setup on creation) wasn't
  ready inside that 2-second window, the picker was left permanently
  unresponsive to input no matter how much longer you then waited or
  clicked. It now wires up its handlers immediately and resolves the
  bridge fresh at the moment you actually click or press a key, so timing
  no longer matters.

## 2.9.4

- Fixed a bug where saving Settings always logged (and on Windows, always
  attempted) an OS-level autostart registration change, even when "Launch
  automatically when I sign in" hadn't actually been touched. On Windows
  specifically this made *every* settings save log a spurious
  `could not update start-at-login: ... (os error 2)` warning, because the
  underlying autostart crate's `disable()` deletes a registry value that
  was never written in the first place whenever autostart is already off.
  `save_config` now only calls into the OS autostart API when the setting
  actually changed.

## 2.9.3

- Removed the search box's idle animations entirely (Default's focus
  scan-sweep, and the continuous glow/sway/bounce pulse on Default,
  Amber, Emerald, and Pink) instead of just optimizing them, per direct
  feedback that they weren't wanted at all - not just that they felt
  heavy. The search box now just sits still until you interact with it;
  its static focus glow (the border/box-shadow on `:focus-within`) is
  unchanged.

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
