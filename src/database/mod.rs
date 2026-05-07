//! Database module - Compatible with cc-switch SQLite schema
//!
//! Uses the same database format as cc-switch for seamless migration

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub app_type: String,
    pub is_healthy: bool,
    pub consecutive_failures: i32,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_error: Option<String>,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub date: String,
    pub provider_id: String,
    pub model: String,
    pub request_count: i64,
    pub success_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: i64,
}

/// Request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub request_id: String,
    pub provider_id: String,
    pub app_type: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_cost_usd: f64,
    pub latency_ms: i64,
    pub first_token_ms: Option<i64>,
    pub status_code: i32,
    pub error_message: Option<String>,
    pub session_id: Option<String>,
    pub is_streaming: bool,
    pub created_at: i64,
}

/// Database connection
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database at the given path
    pub fn open(path: &PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open cc-switch compatible database
    pub fn open_cc_switch_compatible() -> anyhow::Result<Self> {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cc-switch-pro")
            .join("cc-switch-pro.db");

        Self::open(&path)
    }

    /// Initialize database schema (compatible with cc-switch)
    fn init_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL DEFAULT 'claude',
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                icon TEXT,
                icon_color TEXT,
                meta TEXT NOT NULL DEFAULT '{}',
                is_current BOOLEAN NOT NULL DEFAULT 0,
                in_failover_queue BOOLEAN NOT NULL DEFAULT 0,
                cost_multiplier TEXT NOT NULL DEFAULT '1.0',
                limit_daily_usd TEXT,
                limit_monthly_usd TEXT,
                provider_type TEXT,
                PRIMARY KEY (id, app_type)
            );

            CREATE TABLE IF NOT EXISTS provider_endpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                url TEXT NOT NULL,
                added_at INTEGER,
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS proxy_config (
                app_type TEXT PRIMARY KEY CHECK (app_type IN ('claude','codex','gemini')),
                proxy_enabled INTEGER NOT NULL DEFAULT 0,
                listen_address TEXT NOT NULL DEFAULT '127.0.0.1',
                listen_port INTEGER NOT NULL DEFAULT 16789,
                enable_logging INTEGER NOT NULL DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 0,
                auto_failover_enabled INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 3,
                streaming_first_byte_timeout INTEGER NOT NULL DEFAULT 60,
                streaming_idle_timeout INTEGER NOT NULL DEFAULT 120,
                non_streaming_timeout INTEGER NOT NULL DEFAULT 600,
                circuit_failure_threshold INTEGER NOT NULL DEFAULT 4,
                circuit_success_threshold INTEGER NOT NULL DEFAULT 2,
                circuit_timeout_seconds INTEGER NOT NULL DEFAULT 60,
                circuit_error_rate_threshold REAL NOT NULL DEFAULT 0.6,
                circuit_min_requests INTEGER NOT NULL DEFAULT 10,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                live_takeover_active INTEGER NOT NULL DEFAULT 0,
                default_cost_multiplier TEXT NOT NULL DEFAULT '1',
                pricing_model_source TEXT NOT NULL DEFAULT 'response'
            );

            CREATE TABLE IF NOT EXISTS provider_health (
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                is_healthy INTEGER NOT NULL DEFAULT 1,
                consecutive_failures INTEGER NOT NULL DEFAULT 0,
                last_success_at TEXT,
                last_failure_at TEXT,
                last_error TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (provider_id, app_type),
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS proxy_request_logs (
                request_id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                input_cost_usd TEXT NOT NULL DEFAULT '0',
                output_cost_usd TEXT NOT NULL DEFAULT '0',
                cache_read_cost_usd TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_usd TEXT NOT NULL DEFAULT '0',
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                latency_ms INTEGER NOT NULL,
                first_token_ms INTEGER,
                duration_ms INTEGER,
                status_code INTEGER NOT NULL,
                error_message TEXT,
                session_id TEXT,
                provider_type TEXT,
                is_streaming INTEGER NOT NULL DEFAULT 0,
                cost_multiplier TEXT NOT NULL DEFAULT '1.0',
                created_at INTEGER NOT NULL,
                request_model TEXT,
                data_source TEXT NOT NULL DEFAULT 'proxy'
            );

            CREATE TABLE IF NOT EXISTS model_pricing (
                model_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                input_cost_per_million TEXT NOT NULL,
                output_cost_per_million TEXT NOT NULL,
                cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
            );

            CREATE TABLE IF NOT EXISTS stream_check_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                app_type TEXT NOT NULL,
                status TEXT NOT NULL,
                success INTEGER NOT NULL,
                message TEXT NOT NULL,
                response_time_ms INTEGER,
                http_status INTEGER,
                model_used TEXT,
                retry_count INTEGER DEFAULT 0,
                tested_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS usage_daily_rollups (
                date TEXT NOT NULL,
                app_type TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, app_type, provider_id, model)
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON proxy_request_logs(provider_id, app_type);
            CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON proxy_request_logs(created_at);
            CREATE INDEX IF NOT EXISTS idx_request_logs_model ON proxy_request_logs(model);
            CREATE INDEX IF NOT EXISTS idx_stream_check_logs_provider ON stream_check_logs(app_type, provider_id, tested_at DESC);
            CREATE INDEX IF NOT EXISTS idx_providers_failover ON providers(app_type, in_failover_queue, sort_index);
        ")?;

        // Insert default proxy config if not exists
        self.conn.execute(
            "INSERT OR IGNORE INTO proxy_config (app_type) VALUES ('claude')",
            [],
        )?;

        Ok(())
    }

    // =========================================================================
    // Provider operations
    // =========================================================================

    /// Get all providers for an app type
    pub fn get_providers(&self, app_type: &str) -> anyhow::Result<Vec<ProviderRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, settings_config, website_url, category, is_current, 
                    in_failover_queue, cost_multiplier, provider_type, notes
             FROM providers WHERE app_type = ?1 ORDER BY sort_index, name"
        )?;

        let providers = stmt.query_map(params![app_type], |row| {
            Ok(ProviderRow {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: row.get(2)?,
                website_url: row.get(3)?,
                category: row.get(4)?,
                is_current: row.get(5)?,
                in_failover_queue: row.get(6)?,
                cost_multiplier: row.get(7)?,
                provider_type: row.get(8)?,
                notes: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    /// Get a provider by ID
    pub fn get_provider(&self, id: &str, app_type: &str) -> anyhow::Result<Option<ProviderRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, settings_config, website_url, category, is_current, 
                    in_failover_queue, cost_multiplier, provider_type, notes
             FROM providers WHERE id = ?1 AND app_type = ?2"
        )?;

        let mut rows = stmt.query_map(params![id, app_type], |row| {
            Ok(ProviderRow {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: row.get(2)?,
                website_url: row.get(3)?,
                category: row.get(4)?,
                is_current: row.get(5)?,
                in_failover_queue: row.get(6)?,
                cost_multiplier: row.get(7)?,
                provider_type: row.get(8)?,
                notes: row.get(9)?,
            })
        })?;

        Ok(rows.next().transpose()?)
    }

    /// Save a provider
    pub fn save_provider(&self, provider: &ProviderRow) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO providers 
             (id, app_type, name, settings_config, website_url, category, 
              is_current, in_failover_queue, cost_multiplier, provider_type, notes, created_at)
             VALUES (?1, 'claude', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, strftime('%s', 'now'))",
            params![
                provider.id,
                provider.name,
                provider.settings_config,
                provider.website_url,
                provider.category,
                provider.is_current,
                provider.in_failover_queue,
                provider.cost_multiplier,
                provider.provider_type,
                provider.notes,
            ],
        )?;
        Ok(())
    }

    /// Delete a provider
    pub fn delete_provider(&self, id: &str, app_type: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )?;
        Ok(())
    }

    /// Set current provider
    pub fn set_current_provider(&self, id: &str, app_type: &str) -> anyhow::Result<()> {
        // First, unset all current
        self.conn.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )?;
        // Then set the selected one
        self.conn.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )?;
        Ok(())
    }

    // =========================================================================
    // Health operations
    // =========================================================================

    /// Update provider health
    pub fn update_health(&self, provider_id: &str, app_type: &str, success: bool, error: Option<&str>) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        
        if success {
            self.conn.execute(
                "INSERT OR REPLACE INTO provider_health 
                 (provider_id, app_type, is_healthy, consecutive_failures, last_success_at, updated_at)
                 VALUES (?1, ?2, 1, 0, ?3, ?3)",
                params![provider_id, app_type, now],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO provider_health (provider_id, app_type, is_healthy, consecutive_failures, last_failure_at, last_error, updated_at)
                 VALUES (?1, ?2, 0, 1, ?3, ?4, ?3)
                 ON CONFLICT(provider_id, app_type) DO UPDATE SET
                   is_healthy = 0,
                   consecutive_failures = consecutive_failures + 1,
                   last_failure_at = ?3,
                   last_error = ?4,
                   updated_at = ?3",
                params![provider_id, app_type, now, error],
            )?;
        }
        Ok(())
    }

    /// Get provider health
    pub fn get_health(&self, provider_id: &str, app_type: &str) -> anyhow::Result<Option<ProviderHealth>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider_id, app_type, is_healthy, consecutive_failures, 
                    last_success_at, last_failure_at, last_error
             FROM provider_health WHERE provider_id = ?1 AND app_type = ?2"
        )?;

        let mut rows = stmt.query_map(params![provider_id, app_type], |row| {
            Ok(ProviderHealth {
                provider_id: row.get(0)?,
                app_type: row.get(1)?,
                is_healthy: row.get::<_, i32>(2)? != 0,
                consecutive_failures: row.get(3)?,
                last_success_at: row.get(4)?,
                last_failure_at: row.get(5)?,
                last_error: row.get(6)?,
            })
        })?;

        Ok(rows.next().transpose()?)
    }

    // =========================================================================
    // Usage tracking
    // =========================================================================

    /// Log a request
    pub fn log_request(&self, log: &RequestLog) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO proxy_request_logs 
             (request_id, provider_id, app_type, model, input_tokens, output_tokens,
              cache_read_tokens, cache_creation_tokens, total_cost_usd, latency_ms,
              first_token_ms, status_code, error_message, session_id, is_streaming, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                log.request_id,
                log.provider_id,
                log.app_type,
                log.model,
                log.input_tokens,
                log.output_tokens,
                log.cache_read_tokens,
                log.cache_creation_tokens,
                log.total_cost_usd.to_string(),
                log.latency_ms,
                log.first_token_ms,
                log.status_code,
                log.error_message,
                log.session_id,
                log.is_streaming as i32,
                log.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get usage statistics for a date range
    pub fn get_usage_stats(&self, app_type: &str, start_date: &str, end_date: &str) -> anyhow::Result<Vec<UsageStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, provider_id, model, request_count, success_count,
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                    total_cost_usd, avg_latency_ms
             FROM usage_daily_rollups 
             WHERE app_type = ?1 AND date >= ?2 AND date <= ?3
             ORDER BY date DESC"
        )?;

        let stats = stmt.query_map(params![app_type, start_date, end_date], |row| {
            Ok(UsageStats {
                date: row.get(0)?,
                provider_id: row.get(1)?,
                model: row.get(2)?,
                request_count: row.get(3)?,
                success_count: row.get(4)?,
                input_tokens: row.get(5)?,
                output_tokens: row.get(6)?,
                cache_read_tokens: row.get(7)?,
                cache_creation_tokens: row.get(8)?,
                total_cost_usd: row.get::<_, String>(9)?.parse().unwrap_or(0.0),
                avg_latency_ms: row.get(10)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
    }

    /// Get total usage summary
    pub fn get_usage_summary(&self, app_type: &str, days: i32) -> anyhow::Result<UsageSummary> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                COALESCE(SUM(request_count), 0) as total_requests,
                COALESCE(SUM(success_count), 0) as total_success,
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(CAST(total_cost_usd AS REAL)), 0) as total_cost,
                COALESCE(AVG(avg_latency_ms), 0) as avg_latency
             FROM usage_daily_rollups 
             WHERE app_type = ?1 AND date >= date('now', '-' || ?2 || ' days')"
        )?;

        let summary = stmt.query_row(params![app_type, days], |row| {
            Ok(UsageSummary {
                total_requests: row.get(0)?,
                total_success: row.get(1)?,
                total_input_tokens: row.get(2)?,
                total_output_tokens: row.get(3)?,
                total_cost_usd: row.get(4)?,
                avg_latency_ms: row.get(5)?,
            })
        })?;

        Ok(summary)
    }

    // =========================================================================
    // Stream check logs
    // =========================================================================

    /// Log a connection test
    pub fn log_stream_check(&self, log: &StreamCheckLog) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO stream_check_logs 
             (provider_id, provider_name, app_type, status, success, message,
              response_time_ms, http_status, model_used, tested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                log.provider_id,
                log.provider_name,
                log.app_type,
                log.status,
                log.success as i32,
                log.message,
                log.response_time_ms,
                log.http_status,
                log.model_used,
                log.tested_at,
            ],
        )?;
        Ok(())
    }

    /// Get recent stream check logs
    pub fn get_stream_check_logs(&self, app_type: &str, limit: i32) -> anyhow::Result<Vec<StreamCheckLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider_id, provider_name, app_type, status, success, message,
                    response_time_ms, http_status, model_used, tested_at
             FROM stream_check_logs 
             WHERE app_type = ?1
             ORDER BY tested_at DESC
             LIMIT ?2"
        )?;

        let logs = stmt.query_map(params![app_type, limit], |row| {
            Ok(StreamCheckLog {
                provider_id: row.get(0)?,
                provider_name: row.get(1)?,
                app_type: row.get(2)?,
                status: row.get(3)?,
                success: row.get::<_, i32>(4)? != 0,
                message: row.get(5)?,
                response_time_ms: row.get(6)?,
                http_status: row.get(7)?,
                model_used: row.get(8)?,
                tested_at: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(logs)
    }

    // =========================================================================
    // Proxy config
    // =========================================================================

    /// Get proxy config
    pub fn get_proxy_config(&self, app_type: &str) -> anyhow::Result<ProxyConfig> {
        let mut stmt = self.conn.prepare(
            "SELECT proxy_enabled, listen_address, listen_port, enable_logging,
                    auto_failover_enabled, max_retries
             FROM proxy_config WHERE app_type = ?1"
        )?;

        let config = stmt.query_row(params![app_type], |row| {
            Ok(ProxyConfig {
                app_type: app_type.to_string(),
                proxy_enabled: row.get::<_, i32>(0)? != 0,
                listen_address: row.get(1)?,
                listen_port: row.get(2)?,
                enable_logging: row.get::<_, i32>(3)? != 0,
                auto_failover_enabled: row.get::<_, i32>(4)? != 0,
                max_retries: row.get(5)?,
            })
        })?;

        Ok(config)
    }

    /// Update proxy config
    pub fn update_proxy_config(&self, config: &ProxyConfig) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE proxy_config SET 
             proxy_enabled = ?2, listen_address = ?3, listen_port = ?4,
             enable_logging = ?5, auto_failover_enabled = ?6, max_retries = ?7,
             updated_at = datetime('now')
             WHERE app_type = ?1",
            params![
                config.app_type,
                config.proxy_enabled as i32,
                config.listen_address,
                config.listen_port,
                config.enable_logging as i32,
                config.auto_failover_enabled as i32,
                config.max_retries,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // Import from cc-switch
    // =========================================================================

    /// Import providers from cc-switch database
    pub fn import_from_cc_switch(&self, cc_switch_db_path: &PathBuf) -> anyhow::Result<i32> {
        let src_conn = Connection::open(cc_switch_db_path)?;
        
        let mut stmt = src_conn.prepare(
            "SELECT id, name, settings_config, website_url, category, 
                    in_failover_queue, cost_multiplier, provider_type, notes
             FROM providers WHERE app_type = 'claude'"
        )?;

        let mut imported = 0;
        let rows = stmt.query_map([], |row| {
            Ok(ProviderRow {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: row.get(2)?,
                website_url: row.get(3)?,
                category: row.get(4)?,
                is_current: false,
                in_failover_queue: row.get(5)?,
                cost_multiplier: row.get(6)?,
                provider_type: row.get(7)?,
                notes: row.get(8)?,
            })
        })?;

        for row in rows {
            let provider = row?;
            // Check if already exists
            if self.get_provider(&provider.id, "claude")?.is_none() {
                self.save_provider(&provider)?;
                imported += 1;
            }
        }

        Ok(imported)
    }
}

/// Provider row from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRow {
    pub id: String,
    pub name: String,
    pub settings_config: String,
    pub website_url: Option<String>,
    pub category: Option<String>,
    pub is_current: bool,
    pub in_failover_queue: bool,
    pub cost_multiplier: String,
    pub provider_type: Option<String>,
    pub notes: Option<String>,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub app_type: String,
    pub proxy_enabled: bool,
    pub listen_address: String,
    pub listen_port: i32,
    pub enable_logging: bool,
    pub auto_failover_enabled: bool,
    pub max_retries: i32,
}

/// Stream check log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCheckLog {
    pub provider_id: String,
    pub provider_name: String,
    pub app_type: String,
    pub status: String,
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<i64>,
    pub http_status: Option<i32>,
    pub model_used: Option<String>,
    pub tested_at: i64,
}

/// Usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_requests: i64,
    pub total_success: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: i64,
}
