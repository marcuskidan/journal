mod apple_notes;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ── Config persistence ──────────────────────────────────────
// Stores the user's chosen entries folder so it survives app restarts.

fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("com.entries.desktop");
    fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    entries_folder: Option<String>,
    github_repo: Option<String>,
}

fn load_config() -> AppConfig {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &AppConfig) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        fs::write(config_path(), json).ok();
    }
}

// ── Tauri commands (callable from JavaScript) ───────────────

/// Returns the saved entries folder path, or null if none is set.
#[tauri::command]
fn get_saved_folder() -> Option<String> {
    load_config().entries_folder
}

/// Saves the chosen folder path so it persists across launches.
#[tauri::command]
fn set_saved_folder(path: String) -> Result<(), String> {
    let mut config = load_config();
    config.entries_folder = Some(path);
    save_config(&config);
    Ok(())
}

/// Lists all .md files inside the entries/ subfolder of the given root.
/// Returns a vec of filenames like ["meeting-notes.md", "journal.md"].
#[tauri::command]
fn list_notebooks(root: String) -> Result<Vec<String>, String> {
    let entries_dir = Path::new(&root).join("entries");
    if !entries_dir.is_dir() {
        return Err(format!("No entries/ folder found in {}", root));
    }

    let mut notebooks: Vec<String> = fs::read_dir(&entries_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && name != "SCHEMA.md" && name != "SYSTEM.md" {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    notebooks.sort();
    Ok(notebooks)
}

/// Lists subdirectories inside entries/ that contain at least one .md file.
/// These are "directory notebooks" where each .md file is a single entry.
/// Returns a vec of directory names like ["apple-notes-2025", "imported"].
#[tauri::command]
fn list_notebook_dirs(root: String) -> Result<Vec<String>, String> {
    let entries_dir = Path::new(&root).join("entries");
    if !entries_dir.is_dir() {
        return Err(format!("No entries/ folder found in {}", root));
    }

    let mut dirs: Vec<String> = fs::read_dir(&entries_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden directories
            if name.starts_with('.') {
                return None;
            }
            // Only include directories that contain at least one .md file
            let has_md = fs::read_dir(&path).ok()?.any(|f| {
                f.ok()
                    .map(|f| f.file_name().to_string_lossy().ends_with(".md"))
                    .unwrap_or(false)
            });
            if has_md { Some(name) } else { None }
        })
        .collect();

    dirs.sort();
    Ok(dirs)
}

/// Lists all .md files inside a specific subdirectory of entries/.
/// Used for directory-based notebooks where each file is one entry.
/// Returns filenames sorted alphabetically.
#[tauri::command]
fn list_dir_entries(root: String, dir_name: String) -> Result<Vec<String>, String> {
    let dir_path = Path::new(&root).join("entries").join(&dir_name);
    if !dir_path.is_dir() {
        return Err(format!("Directory entries/{} not found", dir_name));
    }

    let mut files: Vec<String> = fs::read_dir(&dir_path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    files.sort();
    Ok(files)
}

/// Reads the full text content of a file at the given path.
#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))
}

/// Writes text content to a file, creating it if it doesn't exist.
#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| format!("Failed to write {}: {}", path, e))
}

/// Checks whether a given path contains an entries/ subdirectory.
#[tauri::command]
fn validate_folder(path: String) -> bool {
    Path::new(&path).join("entries").is_dir()
}

/// Check for updates by querying the GitHub Releases API.
/// Compares the latest release tag against the current app version.
/// The repo is read from the app config; falls back to a placeholder.
#[tauri::command]
fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let config = load_config();
    let repo = config
        .github_repo
        .unwrap_or_else(|| "OWNER/REPO".to_string());

    if repo == "OWNER/REPO" || repo.is_empty() {
        return Ok(UpdateCheckResult {
            update_available: false,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: String::new(),
            release_url: String::new(),
            release_notes: String::new(),
            message: "GitHub repository not configured. Set it in the app config.".to_string(),
        });
    }

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        repo
    );

    let output = Command::new("curl")
        .args([
            "-s",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: Entries-Desktop",
            &url,
        ])
        .output()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !output.status.success() {
        return Err("Failed to reach GitHub API".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Invalid API response: {}", e))?;

    let tag = json["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");
    let html_url = json["html_url"].as_str().unwrap_or("").to_string();
    let release_notes = json["body"].as_str().unwrap_or("").to_string();

    let update_available = !tag.is_empty() && tag != current && version_gt(tag, current);

    let message = if tag.is_empty() {
        "Could not determine latest version.".to_string()
    } else if update_available {
        format!("Version {} is available (you have {}).", tag, current)
    } else {
        format!("You're on the latest version ({}).", current)
    };

    Ok(UpdateCheckResult {
        update_available,
        current_version: current.to_string(),
        latest_version: tag.to_string(),
        release_url: html_url,
        release_notes,
        message,
    })
}

/// Save the GitHub repo path to the app config.
#[tauri::command]
fn set_github_repo(repo: String) -> Result<(), String> {
    let mut config = load_config();
    config.github_repo = Some(repo);
    save_config(&config);
    Ok(())
}

/// Get the configured GitHub repo path.
#[tauri::command]
fn get_github_repo() -> Option<String> {
    load_config().github_repo
}

/// Simple semver comparison: returns true if `a` > `b`.
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().max(vb.len()) {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        if x > y {
            return true;
        }
        if x < y {
            return false;
        }
    }
    false
}

#[derive(Serialize, Deserialize)]
struct UpdateCheckResult {
    update_available: bool,
    current_version: String,
    latest_version: String,
    release_url: String,
    release_notes: String,
    message: String,
}

// ── App entry point ─────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_saved_folder,
            set_saved_folder,
            list_notebooks,
            list_notebook_dirs,
            list_dir_entries,
            read_file,
            write_file,
            validate_folder,
            check_for_updates,
            set_github_repo,
            get_github_repo,
            apple_notes::detect_apple_notes,
            apple_notes::preview_apple_notes,
            apple_notes::import_apple_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Entries");
}
