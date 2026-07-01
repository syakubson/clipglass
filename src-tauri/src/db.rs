use rusqlite::{params, Connection};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub ollama_model: String,
    pub retention_days: i64,
    pub whisper_server_url: String,
    pub whisper_server_token: String,
    pub whisper_server_model: String,
    /// e.g. "option+space", "cmd+space", "ctrl+alt+space"
    pub voice_shortcut: String,
    /// Selected microphone device name (empty = default)
    pub selected_microphone: String,
    /// When false, voice shortcut is not registered (default off).
    pub voice_transcription_enabled: bool,
    /// When false, clipboard entries are not auto-tagged (default off).
    pub ai_tagging_enabled: bool,
    /// When false, hide the footer shortcut strip on the clipboard overlay (default on).
    pub overlay_shortcut_hints_enabled: bool,
    // --- NeuralDeep hub integration ---
    /// Base URL of the NeuralDeep hub (OpenAI-compatible), e.g. https://neuraldeep.ru
    pub hub_url: String,
    /// Per-user API token for the hub (Bearer).
    pub hub_token: String,
    /// Chat model id used for tagging via the hub.
    pub hub_chat_model: String,
    /// Use the hub (instead of local Ollama) for clipboard tagging.
    pub hub_tagging_enabled: bool,
    /// Use the hub for voice transcription.
    pub hub_transcribe_enabled: bool,
    /// Enable the hub agent quick-search command palette.
    pub hub_search_enabled: bool,
    // --- Context-aware voice polishing (stolen from opentypeless) ---
    /// Run transcribed voice through the LLM to clean/format it before pasting.
    pub voice_polish_enabled: bool,
    /// Multimodal model used for polishing (must accept images for screenshot context).
    pub voice_polish_model: String,
    /// Send a screenshot of the target window so the model matches surrounding context.
    pub voice_polish_screenshot: bool,
    /// Extra user instructions appended to the polish prompt.
    pub voice_polish_prompt: String,
    /// If non-empty, translate the polished result into this language code (e.g. "en", "ru").
    pub voice_translate_lang: String,
    /// Newline-separated custom terms with exact spellings to preserve.
    pub voice_dictionary: String,
    /// When text is selected in the target app, treat the spoken transcription as
    /// an instruction to apply to that selection (summarize/fix/translate/rewrite).
    pub voice_selected_text: bool,
    /// Render the clipboard board vertically (a tall mini-clipboard docked to the
    /// screen edge) instead of the default horizontal bottom bar.
    pub board_vertical: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelOption {
    pub value: String,
    pub label: String,
    pub memory_gb: f64,
    pub fits: bool,
    pub installed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelCatalog {
    pub total_memory_gb: f64,
    pub recommended_memory_gb: f64,
    pub options: Vec<ModelOption>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content_type: String, // "text", "image", "file"
    pub text_content: Option<String>,
    pub image_data: Option<String>,  // base64-encoded
    pub image_thumb: Option<String>, // base64-encoded thumbnail
    pub source_app: Option<String>,
    pub source_app_icon: Option<String>, // base64-encoded
    pub content_hash: String,
    pub char_count: Option<i64>,
    pub created_at: String,
    pub is_pinned: bool,
    pub collection_id: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Text recognized from an image via on-device OCR (Vision). None for text entries.
    #[serde(default)]
    pub ocr_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryCounts {
    pub total: i64,
    pub unpinned: i64,
    pub pinned: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct OverlayTagCounts {
    pub semantic: Vec<TagCount>,
    pub format: Vec<TagCount>,
    pub has_text: bool,
    pub has_images: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryTaggedPayload {
    pub entry_id: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedApp {
    pub id: i64,
    pub bundle_id: String,
    pub display_name: String,
}

fn lowercase_search_text(text: &str) -> String {
    text.to_lowercase()
}

const FORMAT_TAG_ORDER: &[&str] = &["gif", "jpg", "png"];
const SEMANTIC_TAG_LIMIT: i64 = 8;

fn push_entry_list_filters(
    sql: &mut String,
    param_values: &mut Vec<Box<dyn rusqlite::types::ToSql>>,
    table_prefix: &str,
    collection_id: Option<i64>,
    pinned_only: bool,
    search: Option<&str>,
) {
    if let Some(cid) = collection_id {
        sql.push_str(&format!(" AND {table_prefix}collection_id = ?"));
        param_values.push(Box::new(cid));
    }
    if pinned_only {
        sql.push_str(&format!(" AND {table_prefix}is_pinned = 1"));
    }
    if let Some(q) = search {
        let q_lower = lowercase_search_text(q);
        if !q_lower.is_empty() {
            sql.push_str(&format!(
                " AND ({table_prefix}text_content_search LIKE ? OR {table_prefix}ocr_text LIKE ?)"
            ));
            let pattern = format!("%{q_lower}%");
            param_values.push(Box::new(pattern.clone()));
            param_values.push(Box::new(pattern));
        }
    }
}

fn is_format_tag(tag: &str) -> bool {
    matches!(tag, "gif" | "jpg" | "png")
}

fn push_content_kind_filter(sql: &mut String, table_prefix: &str, content_kind: &str) {
    match content_kind {
        "text" => sql.push_str(&format!(" AND {table_prefix}content_type = 'text'")),
        "image" => sql.push_str(&format!(" AND {table_prefix}content_type = 'image'")),
        _ => {}
    }
}

fn push_entry_tag_filter(
    sql: &mut String,
    param_values: &mut Vec<Box<dyn rusqlite::types::ToSql>>,
    table_prefix: &str,
    tag: &str,
    tag_variants: Option<&[String]>,
) {
    let id_col = format!("{table_prefix}id");
    if is_format_tag(tag) {
        sql.push_str(&format!(
            " AND {table_prefix}content_type = 'image' AND EXISTS (
                SELECT 1 FROM clipboard_tags ct
                WHERE ct.entry_id = {id_col} AND ct.tag = ?
              )"
        ));
        param_values.push(Box::new(tag.to_owned()));
    } else {
        let tags: Vec<String> = match tag_variants {
            Some(variants) if !variants.is_empty() => variants.to_vec(),
            _ => vec![tag.to_owned()],
        };
        let placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        sql.push_str(&format!(
            " AND {table_prefix}content_type = 'text' AND EXISTS (
                SELECT 1 FROM clipboard_tags ct
                WHERE ct.entry_id = {id_col} AND ct.tag IN ({placeholders})
              )"
        ));
        for variant in tags {
            param_values.push(Box::new(variant));
        }
    }
}

fn backfill_text_content_search(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, text_content
         FROM clipboard_entries
         WHERE text_content_search IS NULL
           AND text_content IS NOT NULL",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    for (id, text) in rows {
        tx.execute(
            "UPDATE clipboard_entries SET text_content_search = ?1 WHERE id = ?2",
            params![lowercase_search_text(&text), id],
        )?;
    }
    tx.commit()
}

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_dir: PathBuf) -> Result<Self, rusqlite::Error> {
        std::fs::create_dir_all(&app_dir).ok();
        let db_path = app_dir.join("copyosity.db");
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;
        ",
        )?;

        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                color TEXT,
                sort_order INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS clipboard_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL DEFAULT 'text',
                text_content TEXT,
                image_data BLOB,
                image_thumb BLOB,
                source_app TEXT,
                source_app_icon BLOB,
                content_hash TEXT NOT NULL,
                char_count INTEGER,
                created_at TEXT NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                collection_id INTEGER REFERENCES collections(id) ON DELETE SET NULL,
                text_content_search TEXT,
                ocr_text TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_entries_created_at ON clipboard_entries(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_entries_content_hash ON clipboard_entries(content_hash);
            CREATE INDEX IF NOT EXISTS idx_entries_collection ON clipboard_entries(collection_id);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS clipboard_tags (
                entry_id INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
                tag TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag)
            );

            CREATE TABLE IF NOT EXISTS clipboard_tag_state (
                entry_id INTEGER PRIMARY KEY REFERENCES clipboard_entries(id) ON DELETE CASCADE,
                status TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS excluded_apps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bundle_id TEXT NOT NULL UNIQUE
            );

            CREATE INDEX IF NOT EXISTS idx_clipboard_tags_entry ON clipboard_tags(entry_id);
            CREATE INDEX IF NOT EXISTS idx_clipboard_tags_tag ON clipboard_tags(tag);
            CREATE INDEX IF NOT EXISTS idx_clipboard_tag_state_status ON clipboard_tag_state(status);
        ")?;

        Self::run_migrations(&conn)?;

        #[cfg(target_os = "macos")]
        crate::macos_app::migrate_legacy_excluded_app_names(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Versioned schema migrations via PRAGMA user_version.
    fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        // v1: add ocr_text to clipboard_entries for databases created before it
        // existed in the CREATE TABLE above. New DBs already have the column, so
        // the ALTER fails with a duplicate-column error which we intentionally ignore.
        if version < 1 {
            let _ = conn.execute("ALTER TABLE clipboard_entries ADD COLUMN ocr_text TEXT", []);
            conn.execute_batch("PRAGMA user_version = 1;")?;
        }

        if version < 2 {
            let _ = conn.execute(
                "ALTER TABLE clipboard_entries ADD COLUMN text_content_search TEXT",
                [],
            );
            backfill_text_content_search(conn)?;
            conn.execute_batch("PRAGMA user_version = 2;")?;
        }

        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            _ => Err(err),
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn is_ai_tagging_enabled(&self) -> bool {
        self.get_app_settings()
            .map(|s| s.ai_tagging_enabled)
            .unwrap_or(false)
    }

    pub fn get_app_settings(&self) -> Result<AppSettings, rusqlite::Error> {
        let ollama_model = self
            .get_setting("ollama_model")?
            .unwrap_or_else(|| "qwen3:4b-instruct-2507-q4_K_M".to_string());
        let retention_days = self
            .get_setting("retention_days")?
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|days| matches!(*days, 1 | 7 | 30 | 180))
            .unwrap_or(30);

        let whisper_server_url = self.get_setting("whisper_server_url")?.unwrap_or_default();
        let whisper_server_token = self
            .get_setting("whisper_server_token")?
            .unwrap_or_default();
        let whisper_server_model = self
            .get_setting("whisper_server_model")?
            .unwrap_or_else(|| "whisper-1".to_string());

        let voice_shortcut = self
            .get_setting("voice_shortcut")?
            .unwrap_or_else(|| "option+space".to_string());
        let selected_microphone = self.get_setting("selected_microphone")?.unwrap_or_default();
        let voice_transcription_enabled = self
            .get_setting("voice_transcription_enabled")?
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);
        let ai_tagging_enabled = self
            .get_setting("ai_tagging_enabled")?
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);
        let overlay_shortcut_hints_enabled = self
            .get_setting("overlay_shortcut_hints_enabled")?
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(true);

        let hub_url = self
            .get_setting("hub_url")?
            .unwrap_or_else(|| "https://api.neuraldeep.ru".to_string());
        let hub_token = self.get_setting("hub_token")?.unwrap_or_default();
        let hub_chat_model = self
            .get_setting("hub_chat_model")?
            .unwrap_or_else(|| "gpt-oss-120b".to_string());
        let hub_tagging_enabled = self
            .get_setting("hub_tagging_enabled")?
            .map(|v| v == "true")
            .unwrap_or(false);
        let hub_transcribe_enabled = self
            .get_setting("hub_transcribe_enabled")?
            .map(|v| v == "true")
            .unwrap_or(false);
        let hub_search_enabled = self
            .get_setting("hub_search_enabled")?
            .map(|v| v == "true")
            .unwrap_or(false);

        let voice_polish_enabled = self
            .get_setting("voice_polish_enabled")?
            .map(|v| v == "true")
            .unwrap_or(false);
        let voice_polish_model = self
            .get_setting("voice_polish_model")?
            .unwrap_or_else(|| "qwen3.6-35b-a3b".to_string());
        let voice_polish_screenshot = self
            .get_setting("voice_polish_screenshot")?
            .map(|v| v == "true")
            .unwrap_or(true);
        let voice_polish_prompt = self.get_setting("voice_polish_prompt")?.unwrap_or_default();
        let voice_translate_lang = self
            .get_setting("voice_translate_lang")?
            .unwrap_or_default();
        let voice_dictionary = self.get_setting("voice_dictionary")?.unwrap_or_default();
        let voice_selected_text = self
            .get_setting("voice_selected_text")?
            .map(|v| v == "true")
            .unwrap_or(false);
        let board_vertical = self
            .get_setting("board_vertical")?
            .map(|v| v == "true")
            .unwrap_or(false);

        Ok(AppSettings {
            ollama_model,
            retention_days,
            whisper_server_url,
            whisper_server_token,
            whisper_server_model,
            voice_shortcut,
            selected_microphone,
            voice_transcription_enabled,
            ai_tagging_enabled,
            overlay_shortcut_hints_enabled,
            hub_url,
            hub_token,
            hub_chat_model,
            hub_tagging_enabled,
            hub_transcribe_enabled,
            hub_search_enabled,
            voice_polish_enabled,
            voice_polish_model,
            voice_polish_screenshot,
            voice_polish_prompt,
            voice_translate_lang,
            voice_dictionary,
            voice_selected_text,
            board_vertical,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_app_settings(
        &self,
        ollama_model: Option<&str>,
        retention_days: Option<i64>,
        whisper_server_url: Option<&str>,
        whisper_server_token: Option<&str>,
        whisper_server_model: Option<&str>,
        voice_shortcut: Option<&str>,
        selected_microphone: Option<&str>,
        voice_transcription_enabled: Option<bool>,
        ai_tagging_enabled: Option<bool>,
        overlay_shortcut_hints_enabled: Option<bool>,
        hub_url: Option<&str>,
        hub_token: Option<&str>,
        hub_chat_model: Option<&str>,
        hub_tagging_enabled: Option<bool>,
        hub_transcribe_enabled: Option<bool>,
        hub_search_enabled: Option<bool>,
        voice_polish_enabled: Option<bool>,
        voice_polish_model: Option<&str>,
        voice_polish_screenshot: Option<bool>,
        voice_polish_prompt: Option<&str>,
        voice_translate_lang: Option<&str>,
        voice_dictionary: Option<&str>,
        voice_selected_text: Option<bool>,
        board_vertical: Option<bool>,
    ) -> Result<AppSettings, rusqlite::Error> {
        if let Some(model) = ollama_model {
            self.set_setting("ollama_model", model.trim())?;
        }
        if let Some(days) = retention_days {
            self.set_setting("retention_days", &days.to_string())?;
        }
        if let Some(url) = whisper_server_url {
            self.set_setting("whisper_server_url", url.trim())?;
        }
        if let Some(token) = whisper_server_token {
            self.set_setting("whisper_server_token", token.trim())?;
        }
        if let Some(model) = whisper_server_model {
            self.set_setting("whisper_server_model", model.trim())?;
        }
        if let Some(sc) = voice_shortcut {
            self.set_setting("voice_shortcut", sc.trim())?;
        }
        if let Some(mic) = selected_microphone {
            self.set_setting("selected_microphone", mic.trim())?;
        }
        if let Some(enabled) = voice_transcription_enabled {
            self.set_setting(
                "voice_transcription_enabled",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(enabled) = ai_tagging_enabled {
            self.set_setting("ai_tagging_enabled", if enabled { "true" } else { "false" })?;
        }
        if let Some(enabled) = overlay_shortcut_hints_enabled {
            self.set_setting(
                "overlay_shortcut_hints_enabled",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(url) = hub_url {
            self.set_setting("hub_url", url.trim())?;
        }
        if let Some(token) = hub_token {
            self.set_setting("hub_token", token.trim())?;
        }
        if let Some(model) = hub_chat_model {
            self.set_setting("hub_chat_model", model.trim())?;
        }
        if let Some(enabled) = hub_tagging_enabled {
            self.set_setting(
                "hub_tagging_enabled",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(enabled) = hub_transcribe_enabled {
            self.set_setting(
                "hub_transcribe_enabled",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(enabled) = hub_search_enabled {
            self.set_setting("hub_search_enabled", if enabled { "true" } else { "false" })?;
        }
        if let Some(enabled) = voice_polish_enabled {
            self.set_setting(
                "voice_polish_enabled",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(model) = voice_polish_model {
            self.set_setting("voice_polish_model", model.trim())?;
        }
        if let Some(enabled) = voice_polish_screenshot {
            self.set_setting(
                "voice_polish_screenshot",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(p) = voice_polish_prompt {
            self.set_setting("voice_polish_prompt", p.trim())?;
        }
        if let Some(lang) = voice_translate_lang {
            self.set_setting("voice_translate_lang", lang.trim())?;
        }
        if let Some(dict) = voice_dictionary {
            self.set_setting("voice_dictionary", dict.trim())?;
        }
        if let Some(enabled) = voice_selected_text {
            self.set_setting(
                "voice_selected_text",
                if enabled { "true" } else { "false" },
            )?;
        }
        if let Some(enabled) = board_vertical {
            self.set_setting("board_vertical", if enabled { "true" } else { "false" })?;
        }

        self.get_app_settings()
    }

    /// Returns (id, is_new). When is_new is false, the entry already existed (duplicate hash).
    pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<(i64, bool), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();

        // Check for duplicate by hash
        let existing: Option<i64> = conn.query_row(
            "SELECT id FROM clipboard_entries WHERE content_hash = ?1 ORDER BY created_at DESC LIMIT 1",
            params![entry.content_hash],
            |row| row.get(0),
        ).ok();

        if let Some(id) = existing {
            // Re-copying content that's already in history: bump it to the top
            // so it resurfaces (a clipboard manager must show the latest copy
            // as newest), instead of silently leaving it buried at its old time.
            conn.execute(
                "UPDATE clipboard_entries SET created_at = ?1 WHERE id = ?2",
                params![entry.created_at, id],
            )?;
            // Backfill image_data for entries created before full-size storage was added
            if entry.image_data.is_some() {
                conn.execute(
                    "UPDATE clipboard_entries SET image_data = ?1 WHERE id = ?2 AND image_data IS NULL",
                    params![entry.image_data, id],
                )?;
            }
            return Ok((id, false));
        }

        let text_content_search = entry.text_content.as_deref().map(lowercase_search_text);

        conn.execute(
            "INSERT INTO clipboard_entries (content_type, text_content, text_content_search, image_data, image_thumb, source_app, source_app_icon, content_hash, char_count, created_at, is_pinned, collection_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.content_type,
                entry.text_content,
                text_content_search,
                entry.image_data,
                entry.image_thumb,
                entry.source_app,
                entry.source_app_icon,
                entry.content_hash,
                entry.char_count,
                entry.created_at,
                entry.is_pinned as i32,
                entry.collection_id,
            ],
        )?;

        Ok((conn.last_insert_rowid(), true))
    }

    pub fn get_entries(
        &self,
        limit: i64,
        offset: i64,
        collection_id: Option<i64>,
        pinned_only: bool,
        search: Option<&str>,
        tag: Option<&str>,
        tag_variants: Option<&[String]>,
        content_kind: Option<&str>,
    ) -> Result<Vec<ClipboardEntry>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from(
            "SELECT id, content_type, text_content, NULL as image_data, COALESCE(image_thumb, image_data) as image_thumb, source_app, NULL as source_app_icon, content_hash, char_count, created_at, is_pinned, collection_id,
             COALESCE((SELECT GROUP_CONCAT(tag, '|') FROM clipboard_tags WHERE entry_id = clipboard_entries.id), '') as tags,
             ocr_text
             FROM clipboard_entries WHERE 1=1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        push_entry_list_filters(
            &mut sql,
            &mut param_values,
            "",
            collection_id,
            pinned_only,
            search,
        );

        if let Some(kind) = content_kind {
            push_content_kind_filter(&mut sql, "", kind);
        }
        if let Some(tag) = tag {
            push_entry_tag_filter(&mut sql, &mut param_values, "", tag, tag_variants);
        }

        sql.push_str(" ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?");
        param_values.push(Box::new(limit));
        param_values.push(Box::new(offset));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    text_content: row.get(2)?,
                    image_data: row.get(3)?,
                    image_thumb: row.get(4)?,
                    source_app: row.get(5)?,
                    source_app_icon: row.get(6)?,
                    content_hash: row.get(7)?,
                    char_count: row.get(8)?,
                    created_at: row.get(9)?,
                    is_pinned: row.get::<_, i32>(10)? != 0,
                    collection_id: row.get(11)?,
                    tags: row
                        .get::<_, String>(12)?
                        .split('|')
                        .filter(|tag| !tag.is_empty())
                        .map(|tag| tag.to_string())
                        .collect(),
                    ocr_text: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    pub fn get_overlay_tag_counts(
        &self,
        collection_id: Option<i64>,
        pinned_only: bool,
        search: Option<&str>,
    ) -> Result<OverlayTagCounts, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();

        let mut semantic_sql = String::from(
            "SELECT ct.tag, COUNT(DISTINCT ce.id) AS cnt
             FROM clipboard_tags ct
             INNER JOIN clipboard_entries ce ON ce.id = ct.entry_id
             WHERE ce.content_type = 'text'
               AND ct.tag NOT IN ('code', 'otp', 'token', 'log', 'gif', 'jpg', 'png', 'jpeg')",
        );
        let mut semantic_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        push_entry_list_filters(
            &mut semantic_sql,
            &mut semantic_params,
            "ce.",
            collection_id,
            pinned_only,
            search,
        );
        semantic_sql.push_str(" GROUP BY ct.tag ORDER BY cnt DESC, ct.tag ASC LIMIT ?");
        semantic_params.push(Box::new(SEMANTIC_TAG_LIMIT));

        let semantic_params_ref: Vec<&dyn rusqlite::types::ToSql> =
            semantic_params.iter().map(|p| p.as_ref()).collect();
        let semantic = conn
            .prepare(&semantic_sql)?
            .query_map(semantic_params_ref.as_slice(), |row| {
                Ok(TagCount {
                    tag: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut format_sql = String::from(
            "SELECT ct.tag AS fmt, COUNT(DISTINCT ce.id) AS cnt
             FROM clipboard_entries ce
             INNER JOIN clipboard_tags ct ON ct.entry_id = ce.id
             WHERE ce.content_type = 'image'
               AND ct.tag IN ('gif', 'jpg', 'png')",
        );
        let mut format_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        push_entry_list_filters(
            &mut format_sql,
            &mut format_params,
            "ce.",
            collection_id,
            pinned_only,
            search,
        );
        format_sql.push_str(" GROUP BY ct.tag ORDER BY cnt DESC, ct.tag ASC");

        let format_params_ref: Vec<&dyn rusqlite::types::ToSql> =
            format_params.iter().map(|p| p.as_ref()).collect();
        let format_rows = conn
            .prepare(&format_sql)?
            .query_map(format_params_ref.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let format_map: std::collections::HashMap<String, i64> = format_rows.into_iter().collect();
        let format = FORMAT_TAG_ORDER
            .iter()
            .filter_map(|tag| {
                format_map.get(*tag).map(|count| TagCount {
                    tag: (*tag).to_owned(),
                    count: *count,
                })
            })
            .collect();

        let mut kind_sql = String::from(
            "SELECT
                EXISTS(SELECT 1 FROM clipboard_entries WHERE content_type = 'text'",
        );
        let mut kind_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        push_entry_list_filters(
            &mut kind_sql,
            &mut kind_params,
            "",
            collection_id,
            pinned_only,
            search,
        );
        kind_sql.push_str(
            "),
                EXISTS(SELECT 1 FROM clipboard_entries WHERE content_type = 'image'",
        );
        push_entry_list_filters(
            &mut kind_sql,
            &mut kind_params,
            "",
            collection_id,
            pinned_only,
            search,
        );
        kind_sql.push(')');

        let kind_params_ref: Vec<&dyn rusqlite::types::ToSql> =
            kind_params.iter().map(|p| p.as_ref()).collect();
        let (has_text, has_images): (i32, i32) =
            conn.query_row(&kind_sql, kind_params_ref.as_slice(), |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;

        Ok(OverlayTagCounts {
            semantic,
            format,
            has_text: has_text != 0,
            has_images: has_images != 0,
        })
    }

    pub fn has_entry_with_content_hash(&self, hash: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(1) FROM clipboard_entries WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn delete_entry(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_entries WHERE id = ?1", params![id])?;
        let unpinned_remaining: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_entries WHERE is_pinned = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(unpinned_remaining == 0)
    }

    pub fn pin_entry(&self, id: i64, pinned: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_entries SET is_pinned = ?1 WHERE id = ?2",
            params![pinned as i32, id],
        )?;
        Ok(())
    }

    pub fn set_collection(
        &self,
        entry_id: i64,
        collection_id: Option<i64>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_entries SET collection_id = ?1 WHERE id = ?2",
            params![collection_id, entry_id],
        )?;
        Ok(())
    }

    // Collections CRUD
    pub fn get_collections(&self) -> Result<Vec<Collection>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, color, sort_order FROM collections ORDER BY sort_order")?;
        let cols = stmt
            .query_map([], |row| {
                Ok(Collection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    sort_order: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(cols)
    }

    pub fn create_collection(
        &self,
        name: &str,
        color: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO collections (name, color) VALUES (?1, ?2)",
            params![name, color],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_collection(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_entries WHERE is_pinned = 0", [])?;
        Ok(())
    }

    pub fn clear_all_history(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_entries", [])?;
        Ok(())
    }

    pub fn get_history_counts(&self) -> Result<HistoryCounts, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| {
            row.get(0)
        })?;
        let pinned: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_entries WHERE is_pinned = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(HistoryCounts {
            total,
            pinned,
            unpinned: total - pinned,
        })
    }

    pub fn get_excluded_apps(&self) -> Result<Vec<ExcludedApp>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, bundle_id FROM excluded_apps ORDER BY bundle_id COLLATE NOCASE")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(conn);

        let bundle_ids: Vec<&str> = rows
            .iter()
            .map(|(_, bundle_id)| bundle_id.as_str())
            .collect();
        let display_names = crate::macos_app::display_names_for_bundle_ids(&bundle_ids);

        Ok(rows
            .into_iter()
            .zip(display_names)
            .map(|((id, bundle_id), display_name)| ExcludedApp {
                id,
                bundle_id,
                display_name,
            })
            .collect())
    }

    pub fn add_excluded_app(&self, bundle_id: &str) -> Result<bool, rusqlite::Error> {
        let normalized = bundle_id.trim();
        if normalized.is_empty() {
            return Ok(false);
        }
        let conn = self.conn.lock().unwrap();
        let changes = conn.execute(
            "INSERT OR IGNORE INTO excluded_apps (bundle_id) VALUES (?1)",
            params![normalized],
        )?;
        Ok(changes > 0)
    }

    pub fn remove_excluded_app(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM excluded_apps WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn is_app_excluded(&self, bundle_id: &str) -> Result<bool, rusqlite::Error> {
        let normalized = bundle_id.trim();
        if normalized.is_empty() {
            return Ok(false);
        }

        let conn = self.conn.lock().unwrap();
        let exists: Option<i64> = conn
            .query_row(
                "SELECT id FROM excluded_apps WHERE bundle_id = ?1 LIMIT 1",
                params![normalized],
                |row| row.get(0),
            )
            .ok();
        Ok(exists.is_some())
    }

    pub fn set_entry_tags(&self, entry_id: i64, tags: &[String]) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM clipboard_tags WHERE entry_id = ?1",
            params![entry_id],
        )?;

        for tag in tags {
            tx.execute(
                "INSERT OR IGNORE INTO clipboard_tags (entry_id, tag) VALUES (?1, ?2)",
                params![entry_id, tag],
            )?;
        }

        tx.execute(
            "INSERT INTO clipboard_tag_state (entry_id, status) VALUES (?1, 'done')
             ON CONFLICT(entry_id) DO UPDATE SET status = excluded.status",
            params![entry_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn set_entry_tag_state(&self, entry_id: i64, status: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clipboard_tag_state (entry_id, status) VALUES (?1, ?2)
             ON CONFLICT(entry_id) DO UPDATE SET status = excluded.status",
            params![entry_id, status],
        )?;
        Ok(())
    }

    pub fn get_untagged_text_entries(
        &self,
        limit: i64,
    ) -> Result<Vec<(i64, String)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT clipboard_entries.id, clipboard_entries.text_content
             FROM clipboard_entries
             LEFT JOIN clipboard_tags ON clipboard_tags.entry_id = clipboard_entries.id
             LEFT JOIN clipboard_tag_state ON clipboard_tag_state.entry_id = clipboard_entries.id
             WHERE clipboard_entries.content_type = 'text'
               AND clipboard_entries.text_content IS NOT NULL
               AND TRIM(clipboard_entries.text_content) != ''
               AND clipboard_tags.entry_id IS NULL
               AND clipboard_tag_state.entry_id IS NULL
             ORDER BY clipboard_entries.created_at DESC
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    pub fn get_text_entries_for_retag(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(i64, String, Vec<String>)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT clipboard_entries.id,
                    clipboard_entries.text_content,
                    COALESCE((SELECT GROUP_CONCAT(tag, '|')
                              FROM clipboard_tags
                              WHERE entry_id = clipboard_entries.id), '') AS tags
             FROM clipboard_entries
             WHERE clipboard_entries.content_type = 'text'
               AND clipboard_entries.text_content IS NOT NULL
               AND TRIM(clipboard_entries.text_content) != ''
             ORDER BY clipboard_entries.created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let entries = stmt
            .query_map(params![limit, offset], |row| {
                let tags = row.get::<_, String>(2)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    tags.split('|')
                        .filter(|tag| !tag.is_empty())
                        .map(|tag| tag.to_string())
                        .collect(),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    pub fn get_entry_text(&self, entry_id: i64) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT text_content
             FROM clipboard_entries
             WHERE id = ?1
               AND content_type = 'text'
               AND text_content IS NOT NULL",
            params![entry_id],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            _ => Err(err),
        })
    }

    pub fn get_entry_by_id(
        &self,
        entry_id: i64,
    ) -> Result<Option<ClipboardEntry>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, content_type, text_content, image_data, image_thumb, source_app, source_app_icon,
                    content_hash, char_count, created_at, is_pinned, collection_id,
                    COALESCE((SELECT GROUP_CONCAT(tag, '|') FROM clipboard_tags WHERE entry_id = clipboard_entries.id), '') as tags,
                    ocr_text
             FROM clipboard_entries
             WHERE id = ?1",
            params![entry_id],
            |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    text_content: row.get(2)?,
                    image_data: row.get(3)?,
                    image_thumb: row.get(4)?,
                    source_app: row.get(5)?,
                    source_app_icon: row.get(6)?,
                    content_hash: row.get(7)?,
                    char_count: row.get(8)?,
                    created_at: row.get(9)?,
                    is_pinned: row.get::<_, i32>(10)? != 0,
                    collection_id: row.get(11)?,
                    tags: row
                        .get::<_, String>(12)?
                        .split('|')
                        .filter(|tag| !tag.is_empty())
                        .map(|tag| tag.to_string())
                        .collect(),
                    ocr_text: row.get(13)?,
                })
            },
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            _ => Err(err),
        })
    }

    /// Store OCR-recognized text for an image entry.
    pub fn set_ocr_text(&self, entry_id: i64, text: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clipboard_entries SET ocr_text = ?1 WHERE id = ?2",
            params![text, entry_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn cleanup_old_entries(&self, max_age_days: i64) -> Result<u64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            // datetime(created_at) normalizes the stored RFC3339 ('T' + tz) so
            // the comparison is correct — a raw string compare against
            // datetime('now') (space-separated) never matched, so retention
            // cleanup silently never ran.
            "DELETE FROM clipboard_entries WHERE is_pinned = 0 AND datetime(created_at) < datetime('now', ?1)",
            params![format!("-{} days", max_age_days)],
        )?;
        Ok(conn.changes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                color TEXT,
                sort_order INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS clipboard_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL DEFAULT 'text',
                text_content TEXT,
                image_data BLOB,
                image_thumb BLOB,
                source_app TEXT,
                source_app_icon BLOB,
                content_hash TEXT NOT NULL,
                char_count INTEGER,
                created_at TEXT NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                collection_id INTEGER REFERENCES collections(id) ON DELETE SET NULL,
                text_content_search TEXT,
                ocr_text TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_entries_content_hash ON clipboard_entries(content_hash);
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS clipboard_tags (
                entry_id INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
                tag TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag)
            );
            CREATE TABLE IF NOT EXISTS clipboard_tag_state (
                entry_id INTEGER PRIMARY KEY REFERENCES clipboard_entries(id) ON DELETE CASCADE,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS excluded_apps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bundle_id TEXT NOT NULL UNIQUE
            );
        ",
        )
        .unwrap();
        Database::run_migrations(&conn).unwrap();
        Database {
            conn: Mutex::new(conn),
        }
    }

    fn make_entry(text: &str, hash: &str) -> ClipboardEntry {
        ClipboardEntry {
            id: 0,
            content_type: "text".to_string(),
            text_content: Some(text.to_string()),
            image_data: None,
            image_thumb: None,
            source_app: Some("TestApp".to_string()),
            source_app_icon: None,
            content_hash: hash.to_string(),
            char_count: Some(text.len() as i64),
            created_at: chrono::Utc::now().to_rfc3339(),
            is_pinned: false,
            collection_id: None,
            tags: Vec::new(),
            ocr_text: None,
        }
    }

    // --- Insert & Dedup ---

    #[test]
    fn insert_entry_returns_new() {
        let db = test_db();
        let entry = make_entry("hello", "hash1");
        let (id, is_new) = db.insert_entry(&entry).unwrap();
        assert!(id > 0);
        assert!(is_new);
    }

    #[test]
    fn insert_duplicate_hash_returns_existing() {
        let db = test_db();
        let e1 = make_entry("hello", "hash_dup");
        let (id1, new1) = db.insert_entry(&e1).unwrap();
        assert!(new1);

        let e2 = make_entry("hello again", "hash_dup");
        let (id2, new2) = db.insert_entry(&e2).unwrap();
        assert!(!new2);
        assert_eq!(id1, id2);
    }

    #[test]
    fn insert_different_hashes_creates_separate() {
        let db = test_db();
        let (id1, _) = db.insert_entry(&make_entry("a", "h1")).unwrap();
        let (id2, _) = db.insert_entry(&make_entry("b", "h2")).unwrap();
        assert_ne!(id1, id2);
    }

    // --- Get entries ---

    #[test]
    fn get_entries_respects_limit() {
        let db = test_db();
        for i in 0..10 {
            db.insert_entry(&make_entry(&format!("text {}", i), &format!("h{}", i)))
                .unwrap();
        }
        let entries = db
            .get_entries(3, 0, None, false, None, None, None, None)
            .unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn get_entries_with_search() {
        let db = test_db();
        db.insert_entry(&make_entry("rust programming", "h1"))
            .unwrap();
        db.insert_entry(&make_entry("python script", "h2")).unwrap();
        db.insert_entry(&make_entry("rust cargo", "h3")).unwrap();

        let results = db
            .get_entries(50, 0, None, false, Some("rust"), None, None, None)
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn get_entries_pinned_only() {
        let db = test_db();
        let (id1, _) = db.insert_entry(&make_entry("pinned", "h1")).unwrap();
        db.insert_entry(&make_entry("not pinned", "h2")).unwrap();
        db.pin_entry(id1, true).unwrap();

        let pinned = db
            .get_entries(50, 0, None, true, None, None, None, None)
            .unwrap();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].text_content.as_deref(), Some("pinned"));
    }

    // --- Pin / Delete ---

    #[test]
    fn pin_and_unpin_entry() {
        let db = test_db();
        let (id, _) = db.insert_entry(&make_entry("test", "h1")).unwrap();

        db.pin_entry(id, true).unwrap();
        let e = db.get_entry_by_id(id).unwrap().unwrap();
        assert!(e.is_pinned);

        db.pin_entry(id, false).unwrap();
        let e = db.get_entry_by_id(id).unwrap().unwrap();
        assert!(!e.is_pinned);
    }

    #[test]
    fn delete_entry_removes_it() {
        let db = test_db();
        let (id, _) = db.insert_entry(&make_entry("to delete", "h1")).unwrap();
        db.delete_entry(id).unwrap();
        assert!(db.get_entry_by_id(id).unwrap().is_none());
    }

    #[test]
    fn clear_history_keeps_pinned() {
        let db = test_db();
        let (id1, _) = db.insert_entry(&make_entry("pinned", "h1")).unwrap();
        db.insert_entry(&make_entry("not pinned", "h2")).unwrap();
        db.pin_entry(id1, true).unwrap();

        db.clear_history().unwrap();
        let all = db
            .get_entries(50, 0, None, false, None, None, None, None)
            .unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_pinned);
    }

    // --- Tags ---

    #[test]
    fn set_and_get_tags() {
        let db = test_db();
        let (id, _) = db.insert_entry(&make_entry("tagged text", "h1")).unwrap();
        db.set_entry_tags(id, &["rust".to_string(), "code".to_string()])
            .unwrap();

        let entry = db.get_entry_by_id(id).unwrap().unwrap();
        assert!(entry.tags.contains(&"rust".to_string()));
        assert!(entry.tags.contains(&"code".to_string()));
    }

    #[test]
    fn overwrite_tags() {
        let db = test_db();
        let (id, _) = db.insert_entry(&make_entry("text", "h1")).unwrap();
        db.set_entry_tags(id, &["old".to_string()]).unwrap();
        db.set_entry_tags(id, &["new".to_string()]).unwrap();

        let entry = db.get_entry_by_id(id).unwrap().unwrap();
        assert_eq!(entry.tags, vec!["new".to_string()]);
    }

    #[test]
    fn untagged_entries_returned() {
        let db = test_db();
        db.insert_entry(&make_entry("no tags", "h1")).unwrap();
        let (id2, _) = db.insert_entry(&make_entry("has tags", "h2")).unwrap();
        db.set_entry_tags(id2, &["tagged".to_string()]).unwrap();

        let untagged = db.get_untagged_text_entries(50).unwrap();
        assert_eq!(untagged.len(), 1);
        assert_eq!(untagged[0].1, "no tags");
    }

    // --- Settings ---

    #[test]
    fn default_settings() {
        let db = test_db();
        let s = db.get_app_settings().unwrap();
        assert_eq!(s.ollama_model, "qwen3:4b-instruct-2507-q4_K_M");
        assert_eq!(s.retention_days, 30);
    }

    #[test]
    fn update_settings() {
        let db = test_db();
        db.update_app_settings(
            Some("custom-model"),
            Some(7),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let s = db.get_app_settings().unwrap();
        assert_eq!(s.ollama_model, "custom-model");
        assert_eq!(s.retention_days, 7);
    }

    #[test]
    fn invalid_retention_falls_back() {
        let db = test_db();
        db.set_setting("retention_days", "999").unwrap();
        let s = db.get_app_settings().unwrap();
        assert_eq!(s.retention_days, 30); // fallback
    }

    // --- Collections ---

    #[test]
    fn create_and_get_collections() {
        let db = test_db();
        let _id = db.create_collection("Work", Some("#ff0000")).unwrap();
        let cols = db.get_collections().unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].name, "Work");
        assert_eq!(cols[0].color, Some("#ff0000".to_string()));
    }

    #[test]
    fn delete_collection_nullifies_entries() {
        let db = test_db();
        let col_id = db.create_collection("Temp", None).unwrap();
        let (entry_id, _) = db.insert_entry(&make_entry("in collection", "h1")).unwrap();
        db.set_collection(entry_id, Some(col_id)).unwrap();

        db.delete_collection(col_id).unwrap();
        let entry = db.get_entry_by_id(entry_id).unwrap().unwrap();
        assert!(entry.collection_id.is_none());
    }

    // --- Excluded apps ---

    #[test]
    fn exclude_and_check_app() {
        let db = test_db();
        assert!(!db.is_app_excluded("Telegram").unwrap());

        assert!(db.add_excluded_app("Telegram").unwrap());
        assert!(db.is_app_excluded("Telegram").unwrap());

        let apps = db.get_excluded_apps().unwrap();
        assert_eq!(apps.len(), 1);

        db.remove_excluded_app(apps[0].id).unwrap();
        assert!(!db.is_app_excluded("Telegram").unwrap());
    }

    #[test]
    fn exclude_empty_app_is_noop() {
        let db = test_db();
        db.add_excluded_app("  ").unwrap();
        assert_eq!(db.get_excluded_apps().unwrap().len(), 0);
    }

    #[test]
    fn exclude_duplicate_app_is_noop() {
        let db = test_db();
        db.add_excluded_app("Safari").unwrap();
        db.add_excluded_app("Safari").unwrap();
        assert_eq!(db.get_excluded_apps().unwrap().len(), 1);
    }
}
