<p align="center">
  <img src="assets/social-preview.png" width="720" alt="LUMA - One search. Every direction. The open-source spotlight for the desktop.">
</p>

<p align="center">
  <a href="../../releases"><img src="https://img.shields.io/badge/version-2.10.3-cf59e6?style=flat-square" alt="Latest version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-8000ff?style=flat-square" alt="MIT License"></a>
  <a href="../../actions/workflows/ci.yml"><img src="https://github.com/LegacyHeaven/luma/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <img src="https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-3a1f5c?style=flat-square" alt="Linux, macOS, Windows">
  <a href="../../discussions"><img src="https://img.shields.io/badge/discussions-join-8000ff?style=flat-square" alt="GitHub Discussions"></a>
</p>

<p align="center">
  <b>LUMA</b> is a fast, open-source spotlight-style search launcher for
  Linux, macOS and Windows.
</p>

Type a plain query, or a `!bang` and a query to jump straight to one of
70+ search engines (Google, YouTube, GitHub, Wikipedia, Amazon, and
more), or to a file or app on your own machine with `!local` and
`!open`. Add your own engines and apps from Settings, or turn on any of
the built-in ones you want - only Google, Local and Open are on by
default, to keep the list uncluttered out of the box.

### Contents

- [Screenshots](#screenshots)
- [How it runs](#how-it-runs)
- [Features](#features)
- [Download](#download)
- [Docs](#docs)
- [License](#license)

## Screenshots

<p align="center">
  <img src="assets/screenshot-main.png" width="600" alt="LUMA main window - search box with the Google engine selected and the bang/shortcut hint below it.">
  <br><sub>Main window</sub>
</p>

<p align="center">
  <img src="assets/screenshot-spotlight.png" width="500" alt="LUMA spotlight window - a small frameless search bar.">
  <br><sub>Spotlight - summoned anywhere with a keyboard shortcut</sub>
</p>

## How it runs

It runs two ways:

- **Main window** - a normal resizable window, opened from the tray or
  your app launcher.
- **Spotlight** - press a keyboard shortcut (`Alt+Space` by default,
  changeable in Settings) anywhere on your desktop and a small,
  frameless search bar drops in near the top of the screen. Search, and
  it hides itself again.

Results open in your default browser, or in LUMA's own built-in browser
window if you'd rather stay inside the app - your choice, in Settings.

## Features

- **Bangs and custom search engines** - `!yt`, `!gh`, `!wiki`, and
  dozens more out of the box; add your own with any site that has a
  search URL. See the full list on the
  [Bangs and Search Engines](../../wiki/Bangs-and-Search-Engines) wiki
  page.
- **`!local` and `!open`** - search your own machine's files, or launch
  an installed app by name, without leaving the keyboard.
- **Themes and plugins** - install either one with a single click from
  the built-in marketplace, or add your own. Plugins add quick-answer
  bangs like `@time` and `@date` that skip the search entirely.
- **Automatic updates** - LUMA checks for a new version and installs it
  for you. No separate download, no installer to re-run.
- **Advanced logging** - off by default; turn it on in Settings if you
  ever need to troubleshoot something or attach a log to a bug report.
- **Localization** - the interface is available in English and Dutch,
  with more languages on the way.

## Download

Grab a prebuilt executable from the
[Releases page](../../releases) - one file per platform, no installer.

<p align="center">
  <a href="../../releases/latest/download/luma-linux-x64"><img src="https://img.shields.io/badge/Linux-download-3a1f5c?style=for-the-badge&logo=linux&logoColor=white" alt="Download for Linux"></a>
  <a href="../../releases/latest/download/luma-macos-arm64"><img src="https://img.shields.io/badge/macOS%20(Apple%20Silicon)-download-3a1f5c?style=for-the-badge&logo=apple&logoColor=white" alt="Download for macOS (Apple Silicon)"></a>
  <a href="../../releases/latest/download/luma-macos-x64"><img src="https://img.shields.io/badge/macOS%20(Intel)-download-3a1f5c?style=for-the-badge&logo=apple&logoColor=white" alt="Download for macOS (Intel)"></a>
  <a href="../../releases/latest/download/luma-windows-x64.exe"><img src="https://img.shields.io/badge/Windows-download-3a1f5c?style=for-the-badge&logo=windows&logoColor=white" alt="Download for Windows"></a>
</p>

| Platform | File |
|---|---|
| Linux | `luma-linux-x64` |
| macOS (Apple Silicon) | `luma-macos-arm64` |
| macOS (Intel) | `luma-macos-x64` |
| Windows | `luma-windows-x64.exe` |

On Linux and macOS you'll need to make it executable first:

```bash
chmod +x luma-linux-x64
./luma-linux-x64
```

On macOS, don't double-click the app the first time. Right-click it (or
Control-click) and choose "Open" instead, then confirm in the dialog that
pops up. After that first launch, it opens normally from then on.

Full first-run and uninstall details are on the
[Installation](../../wiki/Installation) wiki page.

## Docs

<p align="center">
  <a href="../../wiki"><img src="https://img.shields.io/badge/User%20Guide-open%20the%20wiki-8000ff?style=for-the-badge" alt="User Guide"></a>
  <a href="../../wiki/Contributing"><img src="https://img.shields.io/badge/Developer%20Guide-build%20%26%20contribute-3a1f5c?style=for-the-badge" alt="Developer Guide"></a>
</p>

The [User Guide](../../wiki) covers installing, configuring, theming
and everything else you'll need day to day. Building from source,
writing a plugin or theme, and how LUMA works under the hood all live
in the [Developer Guide](../../wiki/Contributing) instead.

Found a bug or have a feature idea? Open one from the
[Issues](../../issues/new/choose) page. General questions belong in
[Discussions](../../discussions).

## License

[MIT](LICENSE) - do whatever you like with it, forks included.
