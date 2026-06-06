use arboard::Clipboard;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use crate::db::{ClipboardEntry, Database};
use crate::ollama;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif"];

/// Skip decoding very large image files from disk (~20 MB).
const MAX_IMAGE_FILE_BYTES: u64 = 20 * 1024 * 1024;

fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn encode_image_from_rgba(bytes: &[u8], width: usize, height: usize) -> Option<(String, String)> {
    let rgba = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, bytes.to_vec())?;
    encode_image_from_dynamic(&DynamicImage::ImageRgba8(rgba))
}

fn encode_image_from_dynamic(image: &DynamicImage) -> Option<(String, String)> {
    let mut full_buf = Cursor::new(Vec::new());
    image.write_to(&mut full_buf, ImageFormat::Png).ok()?;
    let full_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        full_buf.into_inner(),
    );

    let thumb = image.thumbnail(240, 160);
    let mut thumb_buf = Cursor::new(Vec::new());
    thumb.write_to(&mut thumb_buf, ImageFormat::Png).ok()?;
    let thumb_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        thumb_buf.into_inner(),
    );

    Some((full_b64, thumb_b64))
}

fn encode_image_file(path: &Path) -> Option<(String, String)> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_IMAGE_FILE_BYTES {
        return None;
    }
    let image = image::open(path).ok()?;
    encode_image_from_dynamic(&image)
}

fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    let prefix = if data.len() > 4096 {
        &data[..4096]
    } else {
        data
    };
    hasher.update(prefix);
    hasher.update(data.len().to_le_bytes());
    hex::encode(hasher.finalize())
}

fn hash_file_image(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_IMAGE_FILE_BYTES {
        return None;
    }
    let data = std::fs::read(path).ok()?;
    Some(hash_bytes(&data))
}

fn hash_raster_image(bytes: &[u8], width: usize, height: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.update((width as u64).to_le_bytes());
    hasher.update((height as u64).to_le_bytes());
    hex::encode(hasher.finalize())
}

fn entry_content_hash(base: &str) -> String {
    base.to_string()
}

/// Content fingerprint for dedup when pasteboard changeCount bumps but payload is unchanged.
pub fn probe_clipboard_hash(clipboard: &mut Clipboard) -> Option<String> {
    if let Ok(paths) = clipboard.get().file_list() {
        let image_paths: Vec<PathBuf> = paths.into_iter().filter(|p| is_image_path(p)).collect();
        if let Some(path) = image_paths.first() {
            return hash_file_image(path);
        }
    }

    if let Ok(img) = clipboard.get_image() {
        if !img.bytes.is_empty() {
            return Some(hash_raster_image(&img.bytes, img.width, img.height));
        }
    }

    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            return Some(hash_bytes(text.as_bytes()));
        }
    }

    None
}

fn should_skip_source(db: &Database, source_app: &Option<String>) -> bool {
    match source_app {
        Some(app_name) if app_name == "Copyosity" => true,
        Some(app_name) => db.is_app_excluded(app_name).unwrap_or(false),
        None => false,
    }
}

struct CaptureContext {
    app: AppHandle,
    db: Arc<Database>,
}

impl CaptureContext {
    fn try_image(
        &self,
        image_full_b64: String,
        image_thumb_b64: String,
        base_hash: String,
        source_app: Option<String>,
    ) {
        if should_skip_source(&self.db, &source_app) {
            return;
        }

        let content_hash = entry_content_hash(&base_hash);

        let entry = ClipboardEntry {
            id: 0,
            content_type: "image".to_string(),
            text_content: None,
            image_data: Some(image_full_b64),
            image_thumb: Some(image_thumb_b64),
            source_app,
            source_app_icon: None,
            content_hash,
            char_count: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            is_pinned: false,
            collection_id: None,
            tags: Vec::new(),
        };

        if let Ok((id, is_new)) = self.db.insert_entry(&entry) {
            if is_new {
                let mut saved = entry.clone();
                saved.id = id;
                saved.image_data = None;
                let _ = self.app.emit("clipboard-changed", &saved);
            }
        }
    }

    fn try_text(&self, text: String, base_hash: String, source_app: Option<String>) {
        if should_skip_source(&self.db, &source_app) {
            return;
        }

        let content_hash = entry_content_hash(&base_hash);

        let entry = ClipboardEntry {
            id: 0,
            content_type: "text".to_string(),
            text_content: Some(text.clone()),
            image_data: None,
            image_thumb: None,
            source_app,
            source_app_icon: None,
            content_hash,
            char_count: Some(text.len() as i64),
            created_at: chrono::Utc::now().to_rfc3339(),
            is_pinned: false,
            collection_id: None,
            tags: Vec::new(),
        };

        if let Ok((id, is_new)) = self.db.insert_entry(&entry) {
            if is_new {
                let mut saved = entry.clone();
                saved.id = id;
                let _ = self.app.emit("clipboard-changed", &saved);

                let db = self.db.clone();
                let app = self.app.clone();
                std::thread::spawn(move || {
                    if let Some(tags) = ollama::tag_text(&text) {
                        if db.set_entry_tags(id, &tags).is_ok() {
                            let _ = app.emit("entry-tagged", id);
                        }
                    } else {
                        let _ = db.set_entry_tag_state(id, "skipped");
                    }
                });
            }
        }
    }
}

fn try_capture_from_clipboard(clipboard: &mut Clipboard, ctx: &CaptureContext) {
    let source_app = get_frontmost_app();

    // 1. Copied files (Finder, Desktop) — read real pixels from disk.
    //    Must run before get_image(): macOS also puts a generic file-icon TIFF on the pasteboard.
    if let Ok(paths) = clipboard.get().file_list() {
        let image_paths: Vec<PathBuf> = paths.into_iter().filter(|p| is_image_path(p)).collect();
        if !image_paths.is_empty() {
            for path in image_paths {
                let Some(base_hash) = hash_file_image(&path) else {
                    continue;
                };
                let Some((full_b64, thumb_b64)) = encode_image_file(&path) else {
                    continue;
                };
                ctx.try_image(full_b64, thumb_b64, base_hash, source_app.clone());
            }
            return;
        }
    }

    // 2. Raster image (screenshot to clipboard, Copy Image, etc.)
    if let Ok(img) = clipboard.get_image() {
        if !img.bytes.is_empty() {
            let base_hash = hash_raster_image(&img.bytes, img.width, img.height);
            if let Some((full_b64, thumb_b64)) =
                encode_image_from_rgba(&img.bytes, img.width, img.height)
            {
                ctx.try_image(full_b64, thumb_b64, base_hash, source_app);
                return;
            }
        }
    }

    // 3. Plain text
    if let Ok(text) = clipboard.get_text() {
        if text.is_empty() {
            return;
        }
        let base_hash = hash_bytes(text.as_bytes());
        ctx.try_text(text, base_hash, source_app);
    }
}

pub fn start_clipboard_monitor(app: AppHandle) {
    let db = app.state::<Arc<Database>>().inner().clone();

    std::thread::spawn(move || {
        let mut clipboard = Clipboard::new().expect("Failed to access clipboard");
        #[cfg(target_os = "macos")]
        let mut last_change_count = crate::clipboard_macos::change_count();
        let mut last_content_hash = String::new();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));

            #[cfg(target_os = "macos")]
            {
                let change_count = crate::clipboard_macos::change_count();
                if change_count == last_change_count {
                    continue;
                }
                last_change_count = change_count;

                if crate::clipboard_macos::should_ignore_capture(change_count)
                    || crate::clipboard_macos::is_concealed()
                {
                    continue;
                }
            }

            let Some(probe_hash) = probe_clipboard_hash(&mut clipboard) else {
                continue;
            };
            if probe_hash == last_content_hash {
                continue;
            }
            last_content_hash = probe_hash;

            let ctx = CaptureContext {
                app: app.clone(),
                db: db.clone(),
            };
            try_capture_from_clipboard(&mut clipboard, &ctx);
        }
    });
}

#[cfg(target_os = "macos")]
pub fn get_frontmost_app() -> Option<String> {
    use std::process::Command;
    let output = Command::new("lsappinfo")
        .arg("info")
        .arg("-only")
        .arg("name")
        .arg("-app")
        .arg("front")
        .output()
        .ok()?;
    let out = String::from_utf8_lossy(&output.stdout);
    out.split('"')
        .nth(3)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "windows")]
fn get_frontmost_app() -> Option<String> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_frontmost_app() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use rusqlite::Connection;
    use std::path::Path;
    use std::sync::Mutex;

    fn test_db() -> Database {
        let db = Database {
            conn: Mutex::new(Connection::open_in_memory().unwrap()),
        };
        db.conn.lock().unwrap().execute_batch("
            CREATE TABLE IF NOT EXISTS excluded_apps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bundle_id TEXT NOT NULL UNIQUE
            );
        ").unwrap();
        db
    }

    #[test]
    fn entry_content_hash_uses_base_only() {
        assert_eq!(entry_content_hash("abc123"), "abc123");
        assert_eq!(entry_content_hash("abc123"), entry_content_hash("abc123"));
    }

    #[test]
    fn hash_bytes_is_deterministic() {
        let h1 = hash_bytes(b"hello");
        let h2 = hash_bytes(b"hello");
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_bytes(b"world"));
    }

    #[test]
    fn hash_bytes_distinguishes_same_prefix_different_length() {
        let short = hash_bytes(b"abcdefghijklmnop");
        let long = hash_bytes(b"abcdefghijklmnop-extra");
        assert_ne!(short, long);
    }

    #[test]
    fn hash_raster_image_includes_dimensions() {
        let bytes = vec![0u8; 12];
        let small = hash_raster_image(&bytes, 2, 2);
        let large = hash_raster_image(&bytes, 4, 3);
        assert_ne!(small, large);
    }

    #[test]
    fn is_image_path_checks_extensions() {
        assert!(is_image_path(Path::new("/tmp/photo.PNG")));
        assert!(is_image_path(Path::new("/tmp/photo.jpg")));
        assert!(!is_image_path(Path::new("/tmp/readme.txt")));
        assert!(!is_image_path(Path::new("/tmp/noext")));
    }

    #[test]
    fn should_skip_source_copyosity_and_excluded_apps() {
        let db = test_db();
        assert!(should_skip_source(&db, &Some("Copyosity".to_string())));
        assert!(!should_skip_source(&db, &None));
        assert!(!should_skip_source(&db, &Some("Safari".to_string())));

        db.add_excluded_app("Safari").unwrap();
        assert!(should_skip_source(&db, &Some("Safari".to_string())));
    }
}
