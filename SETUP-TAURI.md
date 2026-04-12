# Entries — Desktop App

A native desktop app for the Entries notebook viewer, built with Tauri v2.

## For users: Download & install

No build tools required. Download the latest release for your platform from the **Releases** page on GitHub, then:

- **Mac**: Open the `.dmg`, drag Entries to Applications, double-click to launch.
- **Windows**: Run the `.exe` installer, then launch Entries from the Start menu.

### First launch

1. The app opens showing an empty library.
2. Click **"Connect folder to save tags"** in the sidebar.
3. Pick your project folder (the one with `entries/` inside it).
4. Done. The app remembers this choice permanently.

### Staying up to date

When you (or someone else) pushes changes to the git repo — updated notebooks, new features in `index.html`, etc. — open **Settings** in the app and click **"Check for updates"**. The app pulls the latest changes from the remote and reloads. No reinstall needed.

You only need to download a new release if the Rust backend (`src-tauri/`) changes, which is rare.

---

## For developers: Building from source

If you need to modify the Rust backend or build the app yourself.

### Prerequisites

**1. Install Rust**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, download and run [rustup-init.exe](https://rustup.rs/) instead.

**2. Platform dependencies**

- **macOS**: Xcode Command Line Tools — `xcode-select --install`
- **Windows**: [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload. [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) is usually pre-installed on Windows 10/11.

**3. Install Tauri CLI**

```bash
cargo install tauri-cli --version "^2"
```

### Development

```bash
# Run in dev mode (hot-reloads HTML changes)
cargo tauri dev

# Build a release binary
cargo tauri build
```

The first build takes 2–5 minutes (compiling Rust dependencies). After that, dev mode launches in ~5 seconds.

Release artifacts are produced at:
- **macOS**: `src-tauri/target/release/bundle/dmg/Entries_*.dmg`
- **Windows**: `src-tauri/target/release/bundle/nsis/Entries_*-setup.exe`

### Releasing a new version

The project includes a GitHub Actions workflow (`.github/workflows/build.yml`) that builds for Mac (Apple Silicon + Intel) and Windows automatically. To publish a release:

```bash
# Tag the version
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions builds all three targets and attaches the `.dmg` and `.exe` files to a new Release on the repo's Releases page. You can also trigger a build manually from the Actions tab.

### Adding app icons

Generate icons from a 1024×1024 PNG:

```bash
cargo tauri icon path/to/your-icon.png
```

### Future: iOS support

Tauri v2 supports iOS via `cargo tauri ios init`, `cargo tauri ios dev`, and `cargo tauri ios build`. Requires macOS with Xcode and an Apple Developer account for device builds.

---

## Project structure

```
SkeletonProject/
├── dist/
│   └── index.html            ← The viewer UI (Tauri frontendDist target)
├── entries/                  ← Your markdown notebooks
│   └── meeting-notes.md
├── src-tauri/                ← Tauri native shell
│   ├── Cargo.toml            ← Rust dependencies
│   ├── tauri.conf.json       ← App config (name, window, permissions)
│   ├── icons/                ← App icons
│   └── src/
│       ├── main.rs           ← Entry point
│       └── lib.rs            ← Rust commands (filesystem, config, git pull)
├── .github/workflows/
│   └── build.yml             ← CI/CD: builds Mac + Windows on version tags
├── start.command             ← Browser fallback launcher (Mac)
└── SETUP-TAURI.md            ← This file
```

## How it works

The `index.html` detects whether it's running inside Tauri (via `window.__TAURI_INTERNALS__`) and branches its filesystem calls:

| Operation | Browser | Desktop app (Tauri) |
|---|---|---|
| Pick folder | `showDirectoryPicker()` | Native OS dialog |
| Remember folder | Not possible (session only) | Saved permanently to app config |
| List notebooks | HTTP directory listing | Rust `fs::read_dir()` |
| Read .md files | `fetch()` | Rust `fs::read_to_string()` |
| Write .md files | File System Access API | Rust `fs::write()` |
| Update app | Manual `git pull` in terminal | In-app "Check for updates" button |

The browser paths are fully preserved — `index.html` is the same file for both environments.
