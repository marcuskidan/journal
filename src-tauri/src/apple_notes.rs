//! Apple Notes importer — reads NoteStore.sqlite, decodes protobuf note content,
//! converts to Markdown, and writes individual files into a directory notebook.

use flate2::read::GzDecoder;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

// ── Constants ──────────────────────────────────────────────────

/// Default location of the Apple Notes database relative to home dir.
const NOTE_FOLDER_PATH: &str = "Library/Group Containers/group.com.apple.notes";
const NOTE_DB: &str = "NoteStore.sqlite";
/// Apple CoreTime epoch offset (seconds between Unix epoch and Apple epoch).
const CORETIME_OFFSET: f64 = 978_307_200.0;

// ── Protobuf types (hand-defined for prost, matching Apple Notes schema) ──

#[derive(Clone, PartialEq, Message)]
pub struct ANDocument {
    #[prost(int32, optional, tag = "2")]
    pub version: Option<i32>,
    #[prost(message, optional, tag = "3")]
    pub note: Option<ANNote>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANNote {
    #[prost(string, optional, tag = "2")]
    pub note_text: Option<String>,
    #[prost(message, repeated, tag = "5")]
    pub attribute_run: Vec<ANAttributeRun>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANAttributeRun {
    #[prost(int32, optional, tag = "1")]
    pub length: Option<i32>,
    #[prost(message, optional, tag = "2")]
    pub paragraph_style: Option<ANParagraphStyle>,
    #[prost(message, optional, tag = "3")]
    pub font: Option<ANFont>,
    #[prost(int32, optional, tag = "5")]
    pub font_weight: Option<i32>,
    #[prost(int32, optional, tag = "6")]
    pub underlined: Option<i32>,
    #[prost(int32, optional, tag = "7")]
    pub strikethrough: Option<i32>,
    #[prost(int32, optional, tag = "8")]
    pub superscript: Option<i32>,
    #[prost(string, optional, tag = "9")]
    pub link: Option<String>,
    #[prost(message, optional, tag = "10")]
    pub color: Option<ANColor>,
    #[prost(message, optional, tag = "12")]
    pub attachment_info: Option<ANAttachmentInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANParagraphStyle {
    #[prost(int32, optional, tag = "1")]
    pub style_type: Option<i32>,
    #[prost(int32, optional, tag = "2")]
    pub alignment: Option<i32>,
    #[prost(int32, optional, tag = "4")]
    pub indent_amount: Option<i32>,
    #[prost(message, optional, tag = "5")]
    pub checklist: Option<ANChecklist>,
    #[prost(int32, optional, tag = "8")]
    pub blockquote: Option<i32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANChecklist {
    #[prost(bytes = "vec", optional, tag = "1")]
    pub uuid: Option<Vec<u8>>,
    #[prost(int32, optional, tag = "2")]
    pub done: Option<i32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANFont {
    #[prost(string, optional, tag = "1")]
    pub font_name: Option<String>,
    #[prost(float, optional, tag = "2")]
    pub point_size: Option<f32>,
    #[prost(int32, optional, tag = "3")]
    pub font_hints: Option<i32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANColor {
    #[prost(float, optional, tag = "1")]
    pub red: Option<f32>,
    #[prost(float, optional, tag = "2")]
    pub green: Option<f32>,
    #[prost(float, optional, tag = "3")]
    pub blue: Option<f32>,
    #[prost(float, optional, tag = "4")]
    pub alpha: Option<f32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ANAttachmentInfo {
    #[prost(string, optional, tag = "1")]
    pub attachment_identifier: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub type_uti: Option<String>,
}

// ── Style type constants (matching Apple Notes enums) ──────────

const STYLE_DEFAULT: i32 = -1;
const STYLE_TITLE: i32 = 0;
const STYLE_HEADING: i32 = 1;
const STYLE_SUBHEADING: i32 = 2;
const STYLE_MONOSPACED: i32 = 4;
const STYLE_DOTTED_LIST: i32 = 100;
const STYLE_DASHED_LIST: i32 = 101;
const STYLE_NUMBERED_LIST: i32 = 102;
const STYLE_CHECKBOX: i32 = 103;

const FONT_WEIGHT_BOLD: i32 = 1;
const FONT_WEIGHT_ITALIC: i32 = 2;
const FONT_WEIGHT_BOLD_ITALIC: i32 = 3;

// ── Data types for import results ──────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub imported: usize,
    pub skipped: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub notebook_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImportConfig {
    pub date_format: String,        // e.g. "YYYY-MM-DD"
    pub include_trashed: bool,
    pub notebook_name: String,      // Name for the output directory notebook
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NotePreview {
    pub title: String,
    pub folder: String,
    pub creation_date: String,
    pub trashed: bool,
}

// ── SQLite helper (spawns sqlite3 CLI, which is pre-installed on macOS) ──

fn query_sqlite(db_path: &str, sql: &str) -> Result<String, String> {
    let output = Command::new("sqlite3")
        .args([db_path, "-json", "-readonly", sql])
        .output()
        .map_err(|e| format!("Failed to run sqlite3: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("sqlite3 error: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn query_sqlite_rows(db_path: &str, sql: &str) -> Result<Vec<serde_json::Value>, String> {
    let raw = query_sqlite(db_path, sql)?;
    if raw.is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str(&raw).map_err(|e| format!("JSON parse error: {}", e))
}

// ── Core logic ─────────────────────────────────────────────────

/// Detect the Apple Notes database and return its path if it exists.
fn find_notes_db() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    let db_path = PathBuf::from(&home).join(NOTE_FOLDER_PATH).join(NOTE_DB);
    if db_path.exists() {
        Some(db_path)
    } else {
        None
    }
}

/// Copy the database to a temp location for safe reading.
fn clone_database(source: &Path) -> Result<PathBuf, String> {
    let tmp_dir = env::temp_dir();
    let dest = tmp_dir.join(NOTE_DB);
    let parent = source.parent().unwrap();

    fs::copy(source, &dest).map_err(|e| {
        if e.raw_os_error() == Some(1) {
            "Full Disk Access is required to read Apple Notes. \
             Open System Settings → Privacy & Security → Full Disk Access, \
             then enable it for this app and try again."
                .to_string()
        } else {
            format!("Failed to copy database: {}", e)
        }
    })?;

    // Also copy WAL files if they exist
    let shm = parent.join(format!("{}-shm", NOTE_DB));
    let wal = parent.join(format!("{}-wal", NOTE_DB));
    if shm.exists() {
        let _ = fs::copy(&shm, tmp_dir.join(format!("{}-shm", NOTE_DB)));
    }
    if wal.exists() {
        let _ = fs::copy(&wal, tmp_dir.join(format!("{}-wal", NOTE_DB)));
    }

    Ok(dest)
}

/// Decode an Apple CoreTime timestamp to a Unix timestamp in milliseconds.
fn decode_time(timestamp: f64) -> i64 {
    if timestamp < 1.0 {
        return chrono_now_ms();
    }
    ((timestamp + CORETIME_OFFSET) * 1000.0) as i64
}

fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Convert a Unix timestamp in milliseconds to a date string.
fn format_date(ms: i64, format: &str) -> String {
    let secs = ms / 1000;
    // Use basic date formatting without chrono dependency
    let days_since_epoch = secs / 86400;
    let (year, month, day) = days_to_ymd(days_since_epoch);

    format
        .replace("YYYY", &format!("{:04}", year))
        .replace("MM", &format!("{:02}", month))
        .replace("DD", &format!("{:02}", day))
}

/// Convert days since Unix epoch to year/month/day.
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

/// Try to decompress bytes using gzip, then zlib, then raw deflate.
/// Returns decompressed bytes on success, or None if all methods fail.
fn try_decompress(bytes: &[u8]) -> Option<Vec<u8>> {
    // Strategy 1: Gzip (traditional Apple Notes format)
    {
        let mut decoder = GzDecoder::new(bytes);
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
            return Some(buf);
        }
    }

    // Strategy 2: Zlib (deflate with zlib header — newer macOS may use this)
    {
        use flate2::read::ZlibDecoder;
        let mut decoder = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
            return Some(buf);
        }
    }

    // Strategy 3: Raw deflate (no header at all)
    {
        use flate2::read::DeflateDecoder;
        let mut decoder = DeflateDecoder::new(bytes);
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
            return Some(buf);
        }
    }

    None
}

/// Decode hex string → decompress → protobuf → ANDocument.
/// Tries multiple decompression strategies and falls back to raw protobuf.
fn decode_note_data(hexdata: &str) -> Result<ANDocument, String> {
    // Hex decode
    let bytes = hex_decode(hexdata)?;

    // Strategy A: Decompress the full blob, then decode protobuf
    if let Some(decompressed) = try_decompress(&bytes) {
        if let Ok(doc) = ANDocument::decode(&decompressed[..]) {
            return Ok(doc);
        }
    }

    // Strategy B: Raw protobuf (no compression — some macOS versions store it this way)
    if let Ok(doc) = ANDocument::decode(&bytes[..]) {
        return Ok(doc);
    }

    // Strategy C: Skip potential header bytes (some blobs have a small envelope)
    // Try skipping 2 or 4 bytes, then decompress + decode
    for skip in [2usize, 4, 8] {
        if bytes.len() > skip {
            if let Some(decompressed) = try_decompress(&bytes[skip..]) {
                if let Ok(doc) = ANDocument::decode(&decompressed[..]) {
                    return Ok(doc);
                }
            }
            // Also try raw protobuf after skipping
            if let Ok(doc) = ANDocument::decode(&bytes[skip..]) {
                return Ok(doc);
            }
        }
    }

    // Build a diagnostic message with the first few bytes for debugging
    let preview_len = bytes.len().min(16);
    let first_bytes: Vec<String> = bytes[..preview_len]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    Err(format!(
        "Could not decode note data ({} bytes). First bytes: [{}]. \
         Tried gzip, zlib, raw deflate, raw protobuf, and header-skip variants.",
        bytes.len(),
        first_bytes.join(" ")
    ))
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Odd-length hex string".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Hex decode error at {}: {}", i, e))
        })
        .collect()
}

/// Convert a decoded ANDocument to Markdown text.
fn note_to_markdown(doc: &ANDocument) -> String {
    let note = match &doc.note {
        Some(n) => n,
        None => return String::new(),
    };

    let note_text = note.note_text.as_deref().unwrap_or("");
    if note_text.is_empty() {
        return String::new();
    }

    // If no attribute runs, just return the plain text
    if note.attribute_run.is_empty() {
        return note_text.to_string();
    }

    // Walk attribute runs and convert to Markdown
    let mut result = String::new();
    let mut offset: usize = 0;
    let mut in_code_block = false;
    let mut list_number: i32 = 0;
    let mut last_list_indent: i32 = -1;
    let chars: Vec<char> = note_text.chars().collect();

    for run in &note.attribute_run {
        let len = run.length.unwrap_or(0) as usize;
        if offset + len > chars.len() {
            break;
        }

        let fragment: String = chars[offset..offset + len].iter().collect();
        offset += len;

        // Check if this fragment starts at a line boundary
        let at_line_start = result.is_empty() || result.ends_with('\n');

        let processed = format_run_with_context(
            &fragment,
            run,
            at_line_start,
            &mut in_code_block,
            &mut list_number,
            &mut last_list_indent,
        );
        result.push_str(&processed);
    }

    // Close any open code block
    if in_code_block {
        result.push_str("```\n");
    }

    result.trim().to_string()
}

fn format_run_with_context(
    fragment: &str,
    run: &ANAttributeRun,
    at_line_start: bool,
    in_code_block: &mut bool,
    list_number: &mut i32,
    last_list_indent: &mut i32,
) -> String {
    // Skip attachment placeholders (they show as special Unicode chars)
    if run.attachment_info.is_some() {
        // If it has a link, format as a link reference
        if let Some(link) = &run.link {
            if !link.starts_with("applenotes:") {
                return format!("[attachment]({})", link);
            }
        }
        return String::new();
    }

    let style_type = run
        .paragraph_style
        .as_ref()
        .and_then(|ps| ps.style_type)
        .unwrap_or(STYLE_DEFAULT);
    let indent = run
        .paragraph_style
        .as_ref()
        .and_then(|ps| ps.indent_amount)
        .unwrap_or(0);
    let blockquote = run
        .paragraph_style
        .as_ref()
        .and_then(|ps| ps.blockquote)
        .unwrap_or(0);

    // Handle code block transitions
    if style_type == STYLE_MONOSPACED && !*in_code_block {
        *in_code_block = true;
        let mut out = String::from("\n```\n");
        out.push_str(fragment);
        return out;
    } else if style_type != STYLE_MONOSPACED && *in_code_block {
        *in_code_block = false;
        let mut out = String::from("```\n");
        out.push_str(&format_run(
            fragment,
            run,
            at_line_start,
            in_code_block,
            list_number,
            last_list_indent,
        ));
        return out;
    }

    if *in_code_block {
        // Inside code block, don't apply formatting
        return fragment.to_string();
    }

    format_run(
        fragment,
        run,
        at_line_start,
        in_code_block,
        list_number,
        last_list_indent,
    )
}

fn format_run(
    fragment: &str,
    run: &ANAttributeRun,
    at_line_start: bool,
    _in_code_block: &mut bool,
    list_number: &mut i32,
    last_list_indent: &mut i32,
) -> String {
    let font_weight = run.font_weight.unwrap_or(0);
    let strikethrough = run.strikethrough.unwrap_or(0);
    let style_type = run
        .paragraph_style
        .as_ref()
        .and_then(|ps| ps.style_type)
        .unwrap_or(STYLE_DEFAULT);
    let indent = run
        .paragraph_style
        .as_ref()
        .and_then(|ps| ps.indent_amount)
        .unwrap_or(0);
    let blockquote = run
        .paragraph_style
        .as_ref()
        .and_then(|ps| ps.blockquote)
        .unwrap_or(0);

    // Apply inline formatting
    let mut text = fragment.to_string();

    // Don't apply formatting to whitespace-only fragments
    if text.trim().is_empty() {
        return text;
    }

    match font_weight {
        FONT_WEIGHT_BOLD => text = format!("**{}**", text),
        FONT_WEIGHT_ITALIC => text = format!("*{}*", text),
        FONT_WEIGHT_BOLD_ITALIC => text = format!("***{}***", text),
        _ => {}
    }

    if strikethrough != 0 {
        text = format!("~~{}~~", text);
    }

    // Apply links
    if let Some(link) = &run.link {
        if !link.starts_with("applenotes:") && link != &text {
            text = format!("[{}]({})", text, link);
        }
    }

    // Apply paragraph-level formatting (only at line start)
    if at_line_start {
        let indent_str = "\t".repeat(indent as usize);
        let bq = if blockquote > 0 { "> " } else { "" };

        // Reset numbered list counter if style or indent changed
        if style_type != STYLE_NUMBERED_LIST || indent as i32 != *last_list_indent {
            if style_type == STYLE_NUMBERED_LIST {
                *list_number = 0;
            }
            *last_list_indent = indent as i32;
        }

        match style_type {
            STYLE_TITLE => text = format!("{}# {}", bq, text),
            STYLE_HEADING => text = format!("{}## {}", bq, text),
            STYLE_SUBHEADING => text = format!("{}### {}", bq, text),
            STYLE_DOTTED_LIST | STYLE_DASHED_LIST => {
                text = format!("{}{}- {}", bq, indent_str, text);
            }
            STYLE_NUMBERED_LIST => {
                *list_number += 1;
                text = format!("{}{}{}. {}", bq, indent_str, list_number, text);
            }
            STYLE_CHECKBOX => {
                let done = run
                    .paragraph_style
                    .as_ref()
                    .and_then(|ps| ps.checklist.as_ref())
                    .and_then(|c| c.done)
                    .unwrap_or(0);
                let check = if done != 0 { "[x]" } else { "[ ]" };
                text = format!("{}{}- {} {}", bq, indent_str, check, text);
            }
            _ => {
                if !bq.is_empty() {
                    text = format!("{}{}", bq, text);
                }
            }
        }
    }

    text
}

/// Sanitize a filename by removing/replacing invalid characters.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

// ── Tauri commands ─────────────────────────────────────────────

/// Check if Apple Notes database exists on this machine.
#[tauri::command]
pub fn detect_apple_notes() -> Result<bool, String> {
    Ok(find_notes_db().is_some())
}

/// Preview what notes are available for import (without importing).
#[tauri::command]
pub fn preview_apple_notes(include_trashed: bool) -> Result<Vec<NotePreview>, String> {
    let db_source = find_notes_db().ok_or("Apple Notes database not found")?;
    let db_path = clone_database(&db_source)?;
    let db_str = db_path.to_str().unwrap();

    // Get entity keys
    let keys_rows = query_sqlite_rows(db_str, "SELECT z_ent, z_name FROM z_primarykey")?;
    let mut keys: HashMap<String, i64> = HashMap::new();
    for row in &keys_rows {
        if let (Some(name), Some(ent)) = (row["Z_NAME"].as_str(), row["Z_ENT"].as_i64()) {
            keys.insert(name.to_string(), ent);
        }
    }

    let note_key = keys.get("ICNote").ok_or("ICNote key not found in database")?;
    let folder_key = keys.get("ICFolder").ok_or("ICFolder key not found in database")?;

    // Get folders
    let folder_rows = query_sqlite_rows(
        db_str,
        &format!(
            "SELECT z_pk, ztitle2, zfoldertype FROM ziccloudsyncingobject WHERE z_ent = {}",
            folder_key
        ),
    )?;

    let mut folder_names: HashMap<i64, String> = HashMap::new();
    let mut trash_folders: Vec<i64> = Vec::new();
    for row in &folder_rows {
        let pk = row["Z_PK"].as_i64().unwrap_or(0);
        let title = row["ZTITLE2"].as_str().unwrap_or("(untitled)");
        let folder_type = row["ZFOLDERTYPE"].as_i64().unwrap_or(0);
        folder_names.insert(pk, title.to_string());
        if folder_type == 1 {
            trash_folders.push(pk);
        }
    }

    // Build trash exclusion clause
    let trash_clause = if !include_trashed && !trash_folders.is_empty() {
        format!(
            " AND zfolder NOT IN ({})",
            trash_folders
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    } else {
        String::new()
    };

    // Get notes
    let note_rows = query_sqlite_rows(
        db_str,
        &format!(
            "SELECT z_pk, ztitle1, zfolder, \
             COALESCE(zcreationdate3, zcreationdate2, zcreationdate1, 0) as creation_ts \
             FROM ziccloudsyncingobject \
             WHERE z_ent = {} AND ztitle1 IS NOT NULL{}",
            note_key, trash_clause
        ),
    )?;

    let mut previews = Vec::new();
    for row in &note_rows {
        let title = row["ZTITLE1"].as_str().unwrap_or("(untitled)").to_string();
        let folder_id = row["ZFOLDER"].as_i64().unwrap_or(0);
        let folder = folder_names
            .get(&folder_id)
            .cloned()
            .unwrap_or_else(|| "Notes".to_string());
        let creation_ts = row["creation_ts"].as_f64().unwrap_or(0.0);
        let creation_ms = decode_time(creation_ts);
        let creation_date = format_date(creation_ms, "YYYY-MM-DD");
        let trashed = trash_folders.contains(&folder_id);

        previews.push(NotePreview {
            title,
            folder,
            creation_date,
            trashed,
        });
    }

    // Clean up temp files
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("sqlite-shm"));
    let _ = fs::remove_file(db_path.with_extension("sqlite-wal"));

    Ok(previews)
}

/// Import Apple Notes into a directory notebook inside entries/.
#[tauri::command]
pub fn import_apple_notes(
    root: String,
    config: ImportConfig,
) -> Result<ImportResult, String> {
    let db_source = find_notes_db().ok_or("Apple Notes database not found")?;
    let db_path = clone_database(&db_source)?;
    let db_str = db_path.to_str().unwrap();

    // Prepare output directory
    let notebook_dir_name = sanitize_filename(&config.notebook_name);
    let output_dir = Path::new(&root)
        .join("entries")
        .join(&notebook_dir_name);
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Get entity keys
    let keys_rows = query_sqlite_rows(db_str, "SELECT z_ent, z_name FROM z_primarykey")?;
    let mut keys: HashMap<String, i64> = HashMap::new();
    for row in &keys_rows {
        if let (Some(name), Some(ent)) = (row["Z_NAME"].as_str(), row["Z_ENT"].as_i64()) {
            keys.insert(name.to_string(), ent);
        }
    }

    let note_key = keys.get("ICNote").ok_or("ICNote key not found")?;
    let folder_key = keys.get("ICFolder").ok_or("ICFolder key not found")?;

    // Get folders and identify trash
    let folder_rows = query_sqlite_rows(
        db_str,
        &format!(
            "SELECT z_pk, ztitle2, zfoldertype FROM ziccloudsyncingobject WHERE z_ent = {}",
            folder_key
        ),
    )?;

    let mut folder_names: HashMap<i64, String> = HashMap::new();
    let mut trash_folders: Vec<i64> = Vec::new();
    for row in &folder_rows {
        let pk = row["Z_PK"].as_i64().unwrap_or(0);
        let title = row["ZTITLE2"].as_str().unwrap_or("Notes");
        let folder_type = row["ZFOLDERTYPE"].as_i64().unwrap_or(0);
        folder_names.insert(pk, title.to_string());
        if folder_type == 1 {
            trash_folders.push(pk);
        }
    }

    // Build trash exclusion
    let trash_clause = if !config.include_trashed && !trash_folders.is_empty() {
        format!(
            " AND zfolder NOT IN ({})",
            trash_folders
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    } else {
        String::new()
    };

    // Query all notes with their protobuf data
    let note_rows = query_sqlite_rows(
        db_str,
        &format!(
            "SELECT nd.z_pk, hex(nd.zdata) as zhexdata, zcso.ztitle1, zcso.zfolder, \
             COALESCE(zcso.zcreationdate3, zcso.zcreationdate2, zcso.zcreationdate1, 0) as creation_ts, \
             COALESCE(zcso.zmodificationdate1, 0) as mod_ts \
             FROM zicnotedata AS nd \
             INNER JOIN ziccloudsyncingobject AS zcso ON zcso.z_pk = nd.znote \
             WHERE zcso.z_ent = {} AND zcso.ztitle1 IS NOT NULL{}",
            note_key, trash_clause
        ),
    )?;

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut duplicates = 0usize;
    let mut failed = 0usize;
    let mut first_error: Option<String> = None;

    for row in &note_rows {
        let title = row["ZTITLE1"].as_str().unwrap_or("Untitled");
        let hexdata = row["zhexdata"].as_str().unwrap_or("");
        let creation_ts = row["creation_ts"].as_f64().unwrap_or(0.0);
        let creation_ms = decode_time(creation_ts);

        if hexdata.is_empty() {
            skipped += 1;
            continue;
        }

        // Build filename with date prefix
        let date_prefix = if config.date_format.is_empty() {
            String::new()
        } else {
            format!("{} ", format_date(creation_ms, &config.date_format))
        };

        let safe_title = sanitize_filename(title);
        let filename = format!("{}{}.md", date_prefix, safe_title);
        let file_path = output_dir.join(&filename);

        // Skip if file already exists (duplicate from previous import)
        if file_path.exists() {
            duplicates += 1;
            continue;
        }

        // Decode and convert
        match decode_note_data(hexdata) {
            Ok(doc) => {
                let markdown = note_to_markdown(&doc);
                if markdown.trim().is_empty() {
                    skipped += 1;
                    continue;
                }

                match fs::write(&file_path, &markdown) {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        let err_msg = format!("Failed to write {}: {}", filename, e);
                        eprintln!("{}", err_msg);
                        if first_error.is_none() {
                            first_error = Some(err_msg);
                        }
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Failed to decode '{}': {}", title, e);
                eprintln!("{}", err_msg);
                if first_error.is_none() {
                    first_error = Some(err_msg);
                }
                failed += 1;
            }
        }
    }

    // Clean up temp files
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(db_path.with_extension("sqlite-shm"));
    let _ = fs::remove_file(db_path.with_extension("sqlite-wal"));

    let mut parts = Vec::new();
    if imported > 0 { parts.push(format!("Imported {}", imported)); }
    if duplicates > 0 { parts.push(format!("{} already existed", duplicates)); }
    if skipped > 0 { parts.push(format!("{} empty", skipped)); }
    if failed > 0 { parts.push(format!("{} failed", failed)); }
    let mut message = if parts.is_empty() {
        "No notes found.".to_string()
    } else {
        parts.join(", ")
    };

    // Append the first error detail so the user can see what went wrong
    if let Some(err) = first_error {
        message.push_str(&format!(". Detail: {}", err));
    }

    Ok(ImportResult {
        success: failed == 0,
        message,
        imported,
        skipped,
        duplicates,
        failed,
        notebook_name: notebook_dir_name,
    })
}
