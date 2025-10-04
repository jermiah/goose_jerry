use crate::config::APP_STRATEGY;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::{Provider, MSG_COUNT_FOR_SESSION_NAME_GENERATION};
use crate::recipe::Recipe;
use crate::session::extension_data::ExtensionData;
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Pool, Sqlite};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{info, warn};
use utoipa::ToSchema;

const CURRENT_SCHEMA_VERSION: i32 = 3;

static SESSION_STORAGE: OnceCell<Arc<SessionStorage>> = OnceCell::const_new();

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: String,
    #[schema(value_type = String)]
    pub working_dir: PathBuf,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub extension_data: ExtensionData,
    pub total_tokens: Option<i32>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub accumulated_total_tokens: Option<i32>,
    pub accumulated_input_tokens: Option<i32>,
    pub accumulated_output_tokens: Option<i32>,
    pub schedule_id: Option<String>,
    pub recipe: Option<Recipe>,
    pub conversation: Option<Conversation>,
    pub message_count: usize,
}

pub struct SessionUpdateBuilder {
    session_id: String,
    description: Option<String>,
    working_dir: Option<PathBuf>,
    extension_data: Option<ExtensionData>,
    total_tokens: Option<Option<i32>>,
    input_tokens: Option<Option<i32>>,
    output_tokens: Option<Option<i32>>,
    accumulated_total_tokens: Option<Option<i32>>,
    accumulated_input_tokens: Option<Option<i32>>,
    accumulated_output_tokens: Option<Option<i32>>,
    schedule_id: Option<Option<String>>,
    recipe: Option<Option<Recipe>>,
}

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInsights {
    /// Total number of sessions
    total_sessions: usize,
    /// Total tokens used across all sessions
    total_tokens: i64,
}

impl SessionUpdateBuilder {
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            description: None,
            working_dir: None,
            extension_data: None,
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            schedule_id: None,
            recipe: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn working_dir(mut self, working_dir: PathBuf) -> Self {
        self.working_dir = Some(working_dir);
        self
    }

    pub fn extension_data(mut self, data: ExtensionData) -> Self {
        self.extension_data = Some(data);
        self
    }

    pub fn total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.total_tokens = Some(tokens);
        self
    }

    pub fn input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.input_tokens = Some(tokens);
        self
    }

    pub fn output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.output_tokens = Some(tokens);
        self
    }

    pub fn accumulated_total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_total_tokens = Some(tokens);
        self
    }

    pub fn accumulated_input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_input_tokens = Some(tokens);
        self
    }

    pub fn accumulated_output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_output_tokens = Some(tokens);
        self
    }

    pub fn schedule_id(mut self, schedule_id: Option<String>) -> Self {
        self.schedule_id = Some(schedule_id);
        self
    }

    pub fn recipe(mut self, recipe: Option<Recipe>) -> Self {
        self.recipe = Some(recipe);
        self
    }

    pub async fn apply(self) -> Result<()> {
        SessionManager::apply_update(self).await
    }
}

pub struct SessionManager;

impl SessionManager {
    pub async fn instance() -> Result<Arc<SessionStorage>> {
        SESSION_STORAGE
            .get_or_try_init(|| async { SessionStorage::new().await.map(Arc::new) })
            .await
            .map(Arc::clone)
    }

    pub async fn create_session(working_dir: PathBuf, description: String) -> Result<Session> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        let storage = Self::instance().await?;

        let mut tx = storage.pool.begin().await?;

        let max_idx = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(CAST(SUBSTR(id, 10) AS INTEGER)) FROM sessions WHERE id LIKE ?",
        )
        .bind(format!("{}_%", today))
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);

        let session_id = format!("{}_{}", today, max_idx + 1);

        sqlx::query(
            r#"
        INSERT INTO sessions (id, description, working_dir, extension_data)
        VALUES (?, ?, ?, '{}')
    "#,
        )
        .bind(&session_id)
        .bind(&description)
        .bind(working_dir.to_string_lossy().as_ref())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Self::get_session(&session_id, false).await
    }

    pub async fn get_session(id: &str, include_messages: bool) -> Result<Session> {
        Self::instance()
            .await?
            .get_session(id, include_messages)
            .await
    }

    pub fn update_session(id: &str) -> SessionUpdateBuilder {
        SessionUpdateBuilder::new(id.to_string())
    }

    async fn apply_update(builder: SessionUpdateBuilder) -> Result<()> {
        Self::instance().await?.apply_update(builder).await
    }

    pub async fn add_message(id: &str, message: &Message) -> Result<()> {
        Self::instance().await?.add_message(id, message).await
    }

    pub async fn replace_conversation(id: &str, conversation: &Conversation) -> Result<()> {
        Self::instance()
            .await?
            .replace_conversation(id, conversation)
            .await
    }

    pub async fn list_sessions() -> Result<Vec<Session>> {
        Self::instance().await?.list_sessions().await
    }

    pub async fn delete_session(id: &str) -> Result<()> {
        Self::instance().await?.delete_session(id).await
    }

    pub async fn get_insights() -> Result<SessionInsights> {
        Self::instance().await?.get_insights().await
    }

    pub async fn maybe_update_description(id: &str, provider: Arc<dyn Provider>) -> Result<()> {
        let session = Self::get_session(id, true).await?;
        let conversation = session
            .conversation
            .ok_or_else(|| anyhow::anyhow!("No messages found"))?;

        let user_message_count = conversation
            .messages()
            .iter()
            .filter(|m| matches!(m.role, Role::User))
            .count();

        if user_message_count <= MSG_COUNT_FOR_SESSION_NAME_GENERATION {
            let description = provider.generate_session_name(&conversation).await?;
            Self::update_session(id)
                .description(description)
                .apply()
                .await
        } else {
            Ok(())
        }
    }

    /// Record the start of a tool execution with operation metadata
    pub async fn record_tool_start(
        session_id: &str,
        tool_name: &str,
        operation_type: Option<&str>,
        file_path: Option<&str>,
    ) -> Result<i64> {
        Self::instance()
            .await?
            .record_tool_start(session_id, tool_name, operation_type, file_path)
            .await
    }

    /// Record the completion of a tool execution
    pub async fn record_tool_complete(
        event_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        Self::instance()
            .await?
            .record_tool_complete(event_id, status, error_message)
            .await
    }

    /// Get tool usage statistics for a session
    pub async fn get_tool_stats(session_id: &str) -> Result<ToolStats> {
        Self::instance().await?.get_tool_stats(session_id).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub total_calls: usize,
    pub successful_calls: usize,
    pub failed_calls: usize,
    pub cancelled_calls: usize,
    pub avg_duration_ms: f64,
    pub calls_by_tool: std::collections::HashMap<String, usize>,
    pub calls_by_operation: std::collections::HashMap<String, usize>,
    pub file_operations: Vec<FileOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub file_path: String,
    pub operation_type: String,
    pub timestamp: String,
}

pub struct SessionStorage {
    pool: Pool<Sqlite>,
}

pub fn ensure_session_dir() -> Result<PathBuf> {
    let data_dir = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .data_dir()
        .join("sessions");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}

fn role_to_string(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: String::new(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            description: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            extension_data: ExtensionData::default(),
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            schedule_id: None,
            recipe: None,
            conversation: None,
            message_count: 0,
        }
    }
}

impl Session {
    pub fn without_messages(mut self) -> Self {
        self.conversation = None;
        self
    }
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Session {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let recipe_json: Option<String> = row.try_get("recipe_json")?;
        let recipe = recipe_json.and_then(|json| serde_json::from_str(&json).ok());

        Ok(Session {
            id: row.try_get("id")?,
            working_dir: PathBuf::from(row.try_get::<String, _>("working_dir")?),
            description: row.try_get("description")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            extension_data: serde_json::from_str(&row.try_get::<String, _>("extension_data")?)
                .unwrap_or_default(),
            total_tokens: row.try_get("total_tokens")?,
            input_tokens: row.try_get("input_tokens")?,
            output_tokens: row.try_get("output_tokens")?,
            accumulated_total_tokens: row.try_get("accumulated_total_tokens")?,
            accumulated_input_tokens: row.try_get("accumulated_input_tokens")?,
            accumulated_output_tokens: row.try_get("accumulated_output_tokens")?,
            schedule_id: row.try_get("schedule_id")?,
            recipe,
            conversation: None,
            message_count: row.try_get("message_count").unwrap_or(0) as usize,
        })
    }
}

impl SessionStorage {
    async fn new() -> Result<Self> {
        let session_dir = ensure_session_dir()?;
        let db_path = session_dir.join("sessions.db");

        let storage = if db_path.exists() {
            Self::open(&db_path).await?
        } else {
            let storage = Self::create(&db_path).await?;

            if let Err(e) = storage.import_legacy(&session_dir).await {
                warn!("Failed to import some legacy sessions: {}", e);
            }

            storage
        };

        Ok(storage)
    }

    async fn get_pool(db_path: &Path, create_if_missing: bool) -> Result<Pool<Sqlite>> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(create_if_missing)
            .busy_timeout(std::time::Duration::from_secs(5))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        sqlx::SqlitePool::connect_with(options).await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to open SQLite database at '{}': {}",
                db_path.display(),
                e
            )
        })
    }

    async fn open(db_path: &Path) -> Result<Self> {
        let pool = Self::get_pool(db_path, false).await?;

        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    async fn create(db_path: &Path) -> Result<Self> {
        let pool = Self::get_pool(db_path, true).await?;

        sqlx::query(
            r#"
            CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
            .bind(CURRENT_SCHEMA_VERSION)
            .execute(&pool)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
                working_dir TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                extension_data TEXT DEFAULT '{}',
                total_tokens INTEGER,
                input_tokens INTEGER,
                output_tokens INTEGER,
                accumulated_total_tokens INTEGER,
                accumulated_input_tokens INTEGER,
                accumulated_output_tokens INTEGER,
                schedule_id TEXT,
                recipe_json TEXT
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id),
                role TEXT NOT NULL,
                content_json TEXT NOT NULL,
                created_timestamp INTEGER NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                tokens INTEGER
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query("CREATE INDEX idx_messages_session ON messages(session_id)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_messages_timestamp ON messages(timestamp)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC)")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    async fn import_legacy(&self, session_dir: &PathBuf) -> Result<()> {
        use crate::session::legacy;

        let sessions = match legacy::list_sessions(session_dir) {
            Ok(sessions) => sessions,
            Err(_) => {
                warn!("No legacy sessions found to import");
                return Ok(());
            }
        };

        if sessions.is_empty() {
            return Ok(());
        }

        let mut imported_count = 0;
        let mut failed_count = 0;

        for (session_name, session_path) in sessions {
            match legacy::load_session(&session_name, &session_path) {
                Ok(session) => match self.import_legacy_session(&session).await {
                    Ok(_) => {
                        imported_count += 1;
                        info!("  ✓ Imported: {}", session_name);
                    }
                    Err(e) => {
                        failed_count += 1;
                        info!("  ✗ Failed to import {}: {}", session_name, e);
                    }
                },
                Err(e) => {
                    failed_count += 1;
                    info!("  ✗ Failed to load {}: {}", session_name, e);
                }
            }
        }

        info!(
            "Import complete: {} successful, {} failed",
            imported_count, failed_count
        );
        Ok(())
    }

    async fn import_legacy_session(&self, session: &Session) -> Result<()> {
        let recipe_json = match &session.recipe {
            Some(recipe) => Some(serde_json::to_string(recipe)?),
            None => None,
        };

        sqlx::query(
            r#"
        INSERT INTO sessions (
            id, description, working_dir, created_at, updated_at, extension_data,
            total_tokens, input_tokens, output_tokens,
            accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
            schedule_id, recipe_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&session.id)
        .bind(&session.description)
        .bind(session.working_dir.to_string_lossy().as_ref())
        .bind(&session.created_at)
        .bind(&session.updated_at)
        .bind(serde_json::to_string(&session.extension_data)?)
        .bind(session.total_tokens)
        .bind(session.input_tokens)
        .bind(session.output_tokens)
        .bind(session.accumulated_total_tokens)
        .bind(session.accumulated_input_tokens)
        .bind(session.accumulated_output_tokens)
        .bind(&session.schedule_id)
        .bind(recipe_json)
        .execute(&self.pool)
        .await?;

        if let Some(conversation) = &session.conversation {
            self.replace_conversation(&session.id, conversation).await?;
        }
        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
        let current_version = self.get_schema_version().await?;

        if current_version < CURRENT_SCHEMA_VERSION {
            info!(
                "Running database migrations from v{} to v{}...",
                current_version, CURRENT_SCHEMA_VERSION
            );

            for version in (current_version + 1)..=CURRENT_SCHEMA_VERSION {
                info!("  Applying migration v{}...", version);
                self.apply_migration(version).await?;
                self.update_schema_version(version).await?;
                info!("  ✓ Migration v{} complete", version);
            }

            info!("All migrations complete");
        }

        Ok(())
    }

    async fn get_schema_version(&self) -> Result<i32> {
        let table_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT name FROM sqlite_master
                WHERE type='table' AND name='schema_version'
            )
        "#,
        )
        .fetch_one(&self.pool)
        .await?;

        if !table_exists {
            return Ok(0);
        }

        let version = sqlx::query_scalar::<_, i32>("SELECT MAX(version) FROM schema_version")
            .fetch_one(&self.pool)
            .await?;

        Ok(version)
    }

    async fn update_schema_version(&self, version: i32) -> Result<()> {
        sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
            .bind(version)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn apply_migration(&self, version: i32) -> Result<()> {
        match version {
            1 => {
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS schema_version (
                        version INTEGER PRIMARY KEY,
                        applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )
                "#,
                )
                .execute(&self.pool)
                .await?;
            }
            2 => {
                // Create tool_events table for tracking tool usage
                sqlx::query(
                    r#"
                    CREATE TABLE tool_events (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        session_id TEXT NOT NULL,
                        tool_name TEXT NOT NULL,
                        status TEXT NOT NULL,
                        error_message TEXT,
                        started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        completed_at TIMESTAMP,
                        duration_ms INTEGER,
                        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
                    )
                "#,
                )
                .execute(&self.pool)
                .await?;

                // Create indexes for efficient querying
                sqlx::query("CREATE INDEX idx_tool_events_session ON tool_events(session_id)")
                    .execute(&self.pool)
                    .await?;
                sqlx::query("CREATE INDEX idx_tool_events_tool_name ON tool_events(tool_name)")
                    .execute(&self.pool)
                    .await?;
                sqlx::query("CREATE INDEX idx_tool_events_status ON tool_events(status)")
                    .execute(&self.pool)
                    .await?;
            }
            3 => {
                // Add operation_type and file_path columns to tool_events for better metrics tracking
                sqlx::query(
                    r#"
                    ALTER TABLE tool_events ADD COLUMN operation_type TEXT
                "#,
                )
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    r#"
                    ALTER TABLE tool_events ADD COLUMN file_path TEXT
                "#,
                )
                .execute(&self.pool)
                .await?;

                // Create index for operation_type for efficient querying
                sqlx::query("CREATE INDEX idx_tool_events_operation ON tool_events(operation_type)")
                    .execute(&self.pool)
                    .await?;
                
                // Create index for file_path for efficient querying
                sqlx::query("CREATE INDEX idx_tool_events_file_path ON tool_events(file_path)")
                    .execute(&self.pool)
                    .await?;
            }
            _ => {
                anyhow::bail!("Unknown migration version: {}", version);
            }
        }

        Ok(())
    }

    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session> {
        let mut session = sqlx::query_as::<_, Session>(
            r#"
        SELECT id, working_dir, description, created_at, updated_at, extension_data,
               total_tokens, input_tokens, output_tokens,
               accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
               schedule_id, recipe_json
        FROM sessions
        WHERE id = ?
    "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if include_messages {
            let conv = self.get_conversation(&session.id).await?;
            session.message_count = conv.messages().len();
            session.conversation = Some(conv);
        } else {
            let count =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages WHERE session_id = ?")
                    .bind(&session.id)
                    .fetch_one(&self.pool)
                    .await? as usize;
            session.message_count = count;
        }

        Ok(session)
    }

    async fn apply_update(&self, builder: SessionUpdateBuilder) -> Result<()> {
        let mut updates = Vec::new();
        let mut query = String::from("UPDATE sessions SET ");

        macro_rules! add_update {
            ($field:expr, $name:expr) => {
                if $field.is_some() {
                    if !updates.is_empty() {
                        query.push_str(", ");
                    }
                    updates.push($name);
                    query.push_str($name);
                    query.push_str(" = ?");
                }
            };
        }

        add_update!(builder.description, "description");
        add_update!(builder.working_dir, "working_dir");
        add_update!(builder.extension_data, "extension_data");
        add_update!(builder.total_tokens, "total_tokens");
        add_update!(builder.input_tokens, "input_tokens");
        add_update!(builder.output_tokens, "output_tokens");
        add_update!(builder.accumulated_total_tokens, "accumulated_total_tokens");
        add_update!(builder.accumulated_input_tokens, "accumulated_input_tokens");
        add_update!(
            builder.accumulated_output_tokens,
            "accumulated_output_tokens"
        );
        add_update!(builder.schedule_id, "schedule_id");
        add_update!(builder.recipe, "recipe_json");

        if updates.is_empty() {
            return Ok(());
        }

        if !updates.is_empty() {
            query.push_str(", ");
        }
        query.push_str("updated_at = datetime('now') WHERE id = ?");

        let mut q = sqlx::query(&query);

        if let Some(desc) = builder.description {
            q = q.bind(desc);
        }
        if let Some(wd) = builder.working_dir {
            q = q.bind(wd.to_string_lossy().to_string());
        }
        if let Some(ed) = builder.extension_data {
            q = q.bind(serde_json::to_string(&ed)?);
        }
        if let Some(tt) = builder.total_tokens {
            q = q.bind(tt);
        }
        if let Some(it) = builder.input_tokens {
            q = q.bind(it);
        }
        if let Some(ot) = builder.output_tokens {
            q = q.bind(ot);
        }
        if let Some(att) = builder.accumulated_total_tokens {
            q = q.bind(att);
        }
        if let Some(ait) = builder.accumulated_input_tokens {
            q = q.bind(ait);
        }
        if let Some(aot) = builder.accumulated_output_tokens {
            q = q.bind(aot);
        }
        if let Some(sid) = builder.schedule_id {
            q = q.bind(sid);
        }
        if let Some(recipe) = builder.recipe {
            let recipe_json = recipe.map(|r| serde_json::to_string(&r)).transpose()?;
            q = q.bind(recipe_json);
        }

        q = q.bind(&builder.session_id);
        q.execute(&self.pool).await?;

        Ok(())
    }

    async fn get_conversation(&self, session_id: &str) -> Result<Conversation> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT role, content_json, created_timestamp FROM messages WHERE session_id = ? ORDER BY timestamp",
        )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for (role_str, content_json, created_timestamp) in rows {
            let role = match role_str.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => continue,
            };

            let content = serde_json::from_str(&content_json)?;
            let message = Message::new(role, created_timestamp, content);
            messages.push(message);
        }

        Ok(Conversation::new_unvalidated(messages))
    }

    async fn add_message(&self, session_id: &str, message: &Message) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO messages (session_id, role, content_json, created_timestamp)
            VALUES (?, ?, ?, ?)
        "#,
        )
        .bind(session_id)
        .bind(role_to_string(&message.role))
        .bind(serde_json::to_string(&message.content)?)
        .bind(message.created)
        .execute(&self.pool)
        .await?;

        sqlx::query("UPDATE sessions SET updated_at = datetime('now') WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn replace_conversation(
        &self,
        session_id: &str,
        conversation: &Conversation,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&mut *tx)
            .await?;

        for message in conversation.messages() {
            sqlx::query(
                r#"
            INSERT INTO messages (session_id, role, content_json, created_timestamp)
            VALUES (?, ?, ?, ?)
        "#,
            )
            .bind(session_id)
            .bind(role_to_string(&message.role))
            .bind(serde_json::to_string(&message.content)?)
            .bind(message.created)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            r#"
        SELECT s.id, s.working_dir, s.description, s.created_at, s.updated_at, s.extension_data,
               s.total_tokens, s.input_tokens, s.output_tokens,
               s.accumulated_total_tokens, s.accumulated_input_tokens, s.accumulated_output_tokens,
               s.schedule_id, s.recipe_json,
               COUNT(m.id) as message_count
        FROM sessions s
        INNER JOIN messages m ON s.id = m.session_id
        GROUP BY s.id
        ORDER BY s.updated_at DESC
    "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?)")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;

        if !exists {
            return Err(anyhow::anyhow!("Session not found"));
        }

        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_insights(&self) -> Result<SessionInsights> {
        let row = sqlx::query_as::<_, (i64, Option<i64>)>(
            r#"
            SELECT COUNT(*) as total_sessions,
                   COALESCE(SUM(COALESCE(accumulated_total_tokens, total_tokens, 0)), 0) as total_tokens
            FROM sessions
            "#,
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(SessionInsights {
            total_sessions: row.0 as usize,
            total_tokens: row.1.unwrap_or(0),
        })
    }

    /// Record the start of a tool execution with operation metadata
    async fn record_tool_start(
        &self,
        session_id: &str,
        tool_name: &str,
        operation_type: Option<&str>,
        file_path: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO tool_events (session_id, tool_name, status, operation_type, file_path)
            VALUES (?, ?, 'running', ?, ?)
            "#,
        )
        .bind(session_id)
        .bind(tool_name)
        .bind(operation_type)
        .bind(file_path)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Record the completion of a tool execution
    async fn record_tool_complete(
        &self,
        event_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tool_events
            SET status = ?,
                error_message = ?,
                completed_at = datetime('now'),
                duration_ms = CAST((julianday(datetime('now')) - julianday(started_at)) * 86400000 AS INTEGER)
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(error_message)
        .bind(event_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get tool usage statistics for a session
    async fn get_tool_stats(&self, session_id: &str) -> Result<ToolStats> {
        // Get overall counts
        let counts = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN status = 'cancelled' THEN 1 ELSE 0 END) as cancelled
            FROM tool_events
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        // Get average duration (only for completed tools)
        let avg_duration: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(duration_ms)
            FROM tool_events
            WHERE session_id = ? AND duration_ms IS NOT NULL
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        // Get counts by tool name
        let tool_counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT tool_name, COUNT(*) as count
            FROM tool_events
            WHERE session_id = ?
            GROUP BY tool_name
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut calls_by_tool = std::collections::HashMap::new();
        for (tool_name, count) in tool_counts {
            calls_by_tool.insert(tool_name, count as usize);
        }

        // Get counts by operation type
        let operation_counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT operation_type, COUNT(*) as count
            FROM tool_events
            WHERE session_id = ? AND operation_type IS NOT NULL
            GROUP BY operation_type
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut calls_by_operation = std::collections::HashMap::new();
        for (operation_type, count) in operation_counts {
            calls_by_operation.insert(operation_type, count as usize);
        }

        // Get file operations with timestamps
        let file_ops = sqlx::query_as::<_, (String, String, String)>(
            r#"
            SELECT file_path, operation_type, started_at
            FROM tool_events
            WHERE session_id = ? AND file_path IS NOT NULL AND operation_type IS NOT NULL
            ORDER BY started_at
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let file_operations = file_ops
            .into_iter()
            .map(|(file_path, operation_type, timestamp)| FileOperation {
                file_path,
                operation_type,
                timestamp,
            })
            .collect();

        Ok(ToolStats {
            total_calls: counts.0 as usize,
            successful_calls: counts.1 as usize,
            failed_calls: counts.2 as usize,
            cancelled_calls: counts.3 as usize,
            avg_duration_ms: avg_duration.unwrap_or(0.0),
            calls_by_tool,
            calls_by_operation,
            file_operations,
        })
    }
}
