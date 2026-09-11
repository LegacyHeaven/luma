# Changelog

All notable changes to Luma are documented here.

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
