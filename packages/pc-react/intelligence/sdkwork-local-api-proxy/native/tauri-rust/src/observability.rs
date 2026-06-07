use crate::error::{LocalApiProxyNativeError, LocalApiProxyNativeResult as Result};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use std::{fs, path::PathBuf, sync::Arc};

const MESSAGE_CAPTURE_ENABLED_KEY: &str = "message_capture_enabled";
const DEFAULT_PAGE_SIZE: u64 = 20;
const MAX_PAGE_SIZE: u64 = 100;

fn to_sql_i64(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        LocalApiProxyNativeError::InvalidOperation(format!(
            "local ai proxy observability value {value} exceeds sqlite integer range"
        ))
    })
}

fn to_sql_optional_i64(value: Option<u64>) -> Result<Option<i64>> {
    value.map(to_sql_i64).transpose()
}

fn from_sql_u64(value: i64, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        LocalApiProxyNativeError::InvalidOperation(format!(
            "local ai proxy observability field {field} returned a negative sqlite integer"
        ))
    })
}

fn from_sql_optional_u64(value: Option<i64>, field: &str) -> Result<Option<u64>> {
    value.map(|entry| from_sql_u64(entry, field)).transpose()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyLoggedMessage {
    pub index: u32,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyRequestLogsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyMessageLogsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyRequestLogRecord {
    pub id: String,
    pub created_at: u64,
    pub route_id: String,
    pub route_name: String,
    pub provider_id: String,
    pub client_protocol: String,
    pub upstream_protocol: String,
    pub endpoint: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    pub total_duration_ms: u64,
    pub total_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub request_message_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyMessageLogRecord {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_log_id: Option<String>,
    pub created_at: u64,
    pub route_id: String,
    pub route_name: String,
    pub provider_id: String,
    pub client_protocol: String,
    pub upstream_protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    pub base_url: String,
    pub message_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_preview: Option<String>,
    pub messages: Vec<LocalAiProxyLoggedMessage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyMessageCaptureSettings {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiProxyPaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub has_more: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LocalAiProxyRequestLogInsert {
    pub id: String,
    pub created_at: u64,
    pub route_id: String,
    pub route_name: String,
    pub provider_id: String,
    pub client_protocol: String,
    pub upstream_protocol: String,
    pub endpoint: String,
    pub status: String,
    pub model_id: Option<String>,
    pub base_url: String,
    pub ttft_ms: Option<u64>,
    pub total_duration_ms: u64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub request_preview: Option<String>,
    pub response_preview: Option<String>,
    pub error: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub response_status: Option<u16>,
    pub messages: Vec<LocalAiProxyLoggedMessage>,
}

#[derive(Clone, Debug)]
pub struct LocalAiProxyObservabilityRepository {
    db_path: Arc<PathBuf>,
}

impl LocalAiProxyObservabilityRepository {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let repository = Self {
            db_path: Arc::new(db_path),
        };
        repository.with_connection(|_| Ok(()))?;
        Ok(repository)
    }

    pub fn db_path_string(&self) -> String {
        self.db_path.to_string_lossy().into_owned()
    }

    pub fn message_capture_settings(&self) -> Result<LocalAiProxyMessageCaptureSettings> {
        self.with_connection(|connection| {
            let row = connection
                .query_row(
                    "SELECT bool_value, updated_at FROM observability_settings WHERE key = ?1",
                    params![MESSAGE_CAPTURE_ENABLED_KEY],
                    |row| {
                        let updated_at = row.get::<_, Option<i64>>(1)?;
                        Ok(LocalAiProxyMessageCaptureSettings {
                            enabled: row.get::<_, i64>(0)? != 0,
                            updated_at: from_sql_optional_u64(updated_at, "updated_at").map_err(
                                |error| {
                                    rusqlite::Error::FromSqlConversionFailure(
                                        1,
                                        rusqlite::types::Type::Integer,
                                        Box::new(error),
                                    )
                                },
                            )?,
                        })
                    },
                )
                .optional()?;

            Ok(row.unwrap_or_default())
        })
    }

    pub fn update_message_capture_settings(
        &self,
        enabled: bool,
        updated_at: u64,
    ) -> Result<LocalAiProxyMessageCaptureSettings> {
        self.with_connection(|connection| {
            let updated_at_sql = to_sql_i64(updated_at)?;
            connection.execute(
                "
                INSERT INTO observability_settings (key, bool_value, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(key) DO UPDATE SET
                  bool_value = excluded.bool_value,
                  updated_at = excluded.updated_at
                ",
                params![
                    MESSAGE_CAPTURE_ENABLED_KEY,
                    if enabled { 1 } else { 0 },
                    updated_at_sql
                ],
            )?;

            Ok(LocalAiProxyMessageCaptureSettings {
                enabled,
                updated_at: Some(updated_at),
            })
        })
    }

    pub fn insert_request_log(&self, insert: LocalAiProxyRequestLogInsert) -> Result<()> {
        self.with_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            let created_at = to_sql_i64(insert.created_at)?;
            let ttft_ms = to_sql_optional_i64(insert.ttft_ms)?;
            let total_duration_ms = to_sql_i64(insert.total_duration_ms)?;
            let total_tokens = to_sql_i64(insert.total_tokens)?;
            let input_tokens = to_sql_i64(insert.input_tokens)?;
            let output_tokens = to_sql_i64(insert.output_tokens)?;
            let cache_tokens = to_sql_i64(insert.cache_tokens)?;
            let request_message_count = to_sql_i64(insert.messages.len() as u64)?;
            transaction.execute(
                "
                INSERT OR REPLACE INTO request_logs (
                  id,
                  created_at,
                  route_id,
                  route_name,
                  provider_id,
                  client_protocol,
                  upstream_protocol,
                  endpoint,
                  status,
                  model_id,
                  base_url,
                  ttft_ms,
                  total_duration_ms,
                  total_tokens,
                  input_tokens,
                  output_tokens,
                  cache_tokens,
                  request_message_count,
                  response_status,
                  request_preview,
                  response_preview,
                  error,
                  request_body,
                  response_body
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                  ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                )
                ",
                params![
                    insert.id,
                    created_at,
                    insert.route_id,
                    insert.route_name,
                    insert.provider_id,
                    insert.client_protocol,
                    insert.upstream_protocol,
                    insert.endpoint,
                    insert.status,
                    insert.model_id,
                    insert.base_url,
                    ttft_ms,
                    total_duration_ms,
                    total_tokens,
                    input_tokens,
                    output_tokens,
                    cache_tokens,
                    request_message_count,
                    insert.response_status,
                    insert.request_preview,
                    insert.response_preview,
                    insert.error,
                    insert.request_body,
                    insert.response_body,
                ],
            )?;

            let capture_enabled = transaction
                .query_row(
                    "SELECT bool_value FROM observability_settings WHERE key = ?1",
                    params![MESSAGE_CAPTURE_ENABLED_KEY],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .unwrap_or(0)
                != 0;

            if capture_enabled && !insert.messages.is_empty() {
                let messages_json = serde_json::to_string(&insert.messages).map_err(|error| {
                    LocalApiProxyNativeError::InvalidOperation(format!(
                        "failed to serialize local ai proxy message log: {error}"
                    ))
                })?;
                transaction.execute(
                    "
                    INSERT OR REPLACE INTO message_logs (
                      id,
                      request_log_id,
                      created_at,
                      route_id,
                      route_name,
                      provider_id,
                      client_protocol,
                      upstream_protocol,
                      model_id,
                      base_url,
                      message_count,
                      preview,
                      response_preview,
                      messages_json
                    ) VALUES (
                      ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
                    )
                    ",
                    params![
                        format!("msg_{}", insert.id),
                        insert.id,
                        created_at,
                        insert.route_id,
                        insert.route_name,
                        insert.provider_id,
                        insert.client_protocol,
                        insert.upstream_protocol,
                        insert.model_id,
                        insert.base_url,
                        request_message_count,
                        insert.request_preview,
                        insert.response_preview,
                        messages_json,
                    ],
                )?;
            }

            transaction.commit()?;
            Ok(())
        })
    }

    pub fn list_request_logs(
        &self,
        query: LocalAiProxyRequestLogsQuery,
    ) -> Result<LocalAiProxyPaginatedResult<LocalAiProxyRequestLogRecord>> {
        self.with_connection(|connection| {
            let normalized_page = normalize_page(query.page);
            let normalized_page_size = normalize_page_size(query.page_size);
            let offset = (normalized_page - 1) * normalized_page_size;
            let (where_sql, parameters) = build_request_logs_where_clause(&query);

            let count_sql = format!("SELECT COUNT(*) FROM request_logs{where_sql}");
            let total =
                connection.query_row(&count_sql, params_from_iter(parameters.iter()), |row| {
                    row.get::<_, i64>(0)
                })?;
            let total = from_sql_u64(total, "total")?;

            let mut list_parameters = parameters.clone();
            list_parameters.push(SqlValue::from(to_sql_i64(normalized_page_size)?));
            list_parameters.push(SqlValue::from(to_sql_i64(offset)?));
            let list_sql = format!(
                "
                SELECT
                  id,
                  created_at,
                  route_id,
                  route_name,
                  provider_id,
                  client_protocol,
                  upstream_protocol,
                  endpoint,
                  status,
                  model_id,
                  base_url,
                  ttft_ms,
                  total_duration_ms,
                  total_tokens,
                  input_tokens,
                  output_tokens,
                  cache_tokens,
                  request_message_count,
                  response_status,
                  request_preview,
                  response_preview,
                  error,
                  request_body,
                  response_body
                FROM request_logs
                {where_sql}
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "
            );
            let mut statement = connection.prepare(&list_sql)?;
            let items = statement
                .query_map(params_from_iter(list_parameters.iter()), |row| {
                    let created_at = row.get::<_, i64>(1)?;
                    let ttft_ms = row.get::<_, Option<i64>>(11)?;
                    let total_duration_ms = row.get::<_, i64>(12)?;
                    let total_tokens = row.get::<_, i64>(13)?;
                    let input_tokens = row.get::<_, i64>(14)?;
                    let output_tokens = row.get::<_, i64>(15)?;
                    let cache_tokens = row.get::<_, i64>(16)?;
                    let request_message_count = row.get::<_, i64>(17)?;
                    Ok(LocalAiProxyRequestLogRecord {
                        id: row.get(0)?,
                        created_at: from_sql_u64(created_at, "created_at").map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                        route_id: row.get(2)?,
                        route_name: row.get(3)?,
                        provider_id: row.get(4)?,
                        client_protocol: row.get(5)?,
                        upstream_protocol: row.get(6)?,
                        endpoint: row.get(7)?,
                        status: row.get(8)?,
                        model_id: row.get(9)?,
                        base_url: row.get(10)?,
                        ttft_ms: from_sql_optional_u64(ttft_ms, "ttft_ms").map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                11,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                        total_duration_ms: from_sql_u64(total_duration_ms, "total_duration_ms")
                            .map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    12,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            })?,
                        total_tokens: from_sql_u64(total_tokens, "total_tokens").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    13,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        prompt_tokens: from_sql_u64(input_tokens, "input_tokens").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    14,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        completion_tokens: from_sql_u64(output_tokens, "output_tokens").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    15,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        input_tokens: from_sql_u64(input_tokens, "input_tokens").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    14,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        output_tokens: from_sql_u64(output_tokens, "output_tokens").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    15,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        cache_tokens: from_sql_u64(cache_tokens, "cache_tokens").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    16,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        request_message_count: from_sql_u64(
                            request_message_count,
                            "request_message_count",
                        )
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                17,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                        response_status: row.get(18)?,
                        request_preview: row.get(19)?,
                        response_preview: row.get(20)?,
                        error: row.get(21)?,
                        request_body: row.get(22)?,
                        response_body: row.get(23)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            Ok(LocalAiProxyPaginatedResult {
                has_more: total > normalized_page.saturating_mul(normalized_page_size),
                items,
                total,
                page: normalized_page,
                page_size: normalized_page_size,
            })
        })
    }

    pub fn list_message_logs(
        &self,
        query: LocalAiProxyMessageLogsQuery,
    ) -> Result<LocalAiProxyPaginatedResult<LocalAiProxyMessageLogRecord>> {
        self.with_connection(|connection| {
            let normalized_page = normalize_page(query.page);
            let normalized_page_size = normalize_page_size(query.page_size);
            let offset = (normalized_page - 1) * normalized_page_size;
            let (where_sql, parameters) = build_message_logs_where_clause(&query);

            let count_sql = format!("SELECT COUNT(*) FROM message_logs{where_sql}");
            let total =
                connection.query_row(&count_sql, params_from_iter(parameters.iter()), |row| {
                    row.get::<_, i64>(0)
                })?;
            let total = from_sql_u64(total, "total")?;

            let mut list_parameters = parameters.clone();
            list_parameters.push(SqlValue::from(to_sql_i64(normalized_page_size)?));
            list_parameters.push(SqlValue::from(to_sql_i64(offset)?));
            let list_sql = format!(
                "
                SELECT
                  id,
                  request_log_id,
                  created_at,
                  route_id,
                  route_name,
                  provider_id,
                  client_protocol,
                  upstream_protocol,
                  model_id,
                  base_url,
                  message_count,
                  preview,
                  response_preview,
                  messages_json
                FROM message_logs
                {where_sql}
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "
            );
            let mut statement = connection.prepare(&list_sql)?;
            let items = statement
                .query_map(params_from_iter(list_parameters.iter()), |row| {
                    let created_at = row.get::<_, i64>(2)?;
                    let message_count = row.get::<_, i64>(10)?;
                    let messages_json: String = row.get(13)?;
                    let messages =
                        serde_json::from_str::<Vec<LocalAiProxyLoggedMessage>>(&messages_json)
                            .map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    messages_json.len(),
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })?;
                    Ok(LocalAiProxyMessageLogRecord {
                        id: row.get(0)?,
                        request_log_id: row.get(1)?,
                        created_at: from_sql_u64(created_at, "created_at").map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                        route_id: row.get(3)?,
                        route_name: row.get(4)?,
                        provider_id: row.get(5)?,
                        client_protocol: row.get(6)?,
                        upstream_protocol: row.get(7)?,
                        model_id: row.get(8)?,
                        base_url: row.get(9)?,
                        message_count: from_sql_u64(message_count, "message_count").map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    10,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        preview: row.get(11)?,
                        response_preview: row.get(12)?,
                        messages,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            Ok(LocalAiProxyPaginatedResult {
                has_more: total > normalized_page.saturating_mul(normalized_page_size),
                items,
                total,
                page: normalized_page,
                page_size: normalized_page_size,
            })
        })
    }

    fn with_connection<T>(&self, operation: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(self.db_path.as_ref())?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", 1)?;
        ensure_schema(&connection)?;
        operation(&connection)
    }
}

fn ensure_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS request_logs (
          id TEXT PRIMARY KEY,
          created_at INTEGER NOT NULL,
          route_id TEXT NOT NULL,
          route_name TEXT NOT NULL,
          provider_id TEXT NOT NULL,
          client_protocol TEXT NOT NULL,
          upstream_protocol TEXT NOT NULL,
          endpoint TEXT NOT NULL,
          status TEXT NOT NULL,
          model_id TEXT,
          base_url TEXT NOT NULL,
          ttft_ms INTEGER,
          total_duration_ms INTEGER NOT NULL,
          total_tokens INTEGER NOT NULL,
          input_tokens INTEGER NOT NULL,
          output_tokens INTEGER NOT NULL,
          cache_tokens INTEGER NOT NULL,
          request_message_count INTEGER NOT NULL,
          response_status INTEGER,
          request_preview TEXT,
          response_preview TEXT,
          error TEXT,
          request_body TEXT,
          response_body TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_request_logs_provider_id ON request_logs(provider_id);
        CREATE INDEX IF NOT EXISTS idx_request_logs_route_id ON request_logs(route_id);
        CREATE INDEX IF NOT EXISTS idx_request_logs_model_id ON request_logs(model_id);
        CREATE INDEX IF NOT EXISTS idx_request_logs_status ON request_logs(status);

        CREATE TABLE IF NOT EXISTS message_logs (
          id TEXT PRIMARY KEY,
          request_log_id TEXT,
          created_at INTEGER NOT NULL,
          route_id TEXT NOT NULL,
          route_name TEXT NOT NULL,
          provider_id TEXT NOT NULL,
          client_protocol TEXT NOT NULL,
          upstream_protocol TEXT NOT NULL,
          model_id TEXT,
          base_url TEXT NOT NULL,
          message_count INTEGER NOT NULL,
          preview TEXT,
          response_preview TEXT,
          messages_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_message_logs_created_at ON message_logs(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_message_logs_provider_id ON message_logs(provider_id);
        CREATE INDEX IF NOT EXISTS idx_message_logs_route_id ON message_logs(route_id);
        CREATE INDEX IF NOT EXISTS idx_message_logs_model_id ON message_logs(model_id);

        CREATE TABLE IF NOT EXISTS observability_settings (
          key TEXT PRIMARY KEY,
          bool_value INTEGER NOT NULL,
          updated_at INTEGER
        );
        ",
    )?;
    Ok(())
}

fn normalize_page(value: Option<u64>) -> u64 {
    value.filter(|entry| *entry > 0).unwrap_or(1)
}

fn normalize_page_size(value: Option<u64>) -> u64 {
    value
        .filter(|entry| *entry > 0)
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(MAX_PAGE_SIZE)
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
}

fn push_exact_filter(
    where_parts: &mut Vec<String>,
    parameters: &mut Vec<SqlValue>,
    column: &str,
    value: Option<&str>,
) {
    if let Some(value) = normalize_optional_text(value) {
        where_parts.push(format!("{column} = ?"));
        parameters.push(SqlValue::from(value));
    }
}

fn build_request_logs_where_clause(
    query: &LocalAiProxyRequestLogsQuery,
) -> (String, Vec<SqlValue>) {
    let mut where_parts = Vec::new();
    let mut parameters = Vec::new();
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "provider_id",
        query.provider_id.as_deref(),
    );
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "model_id",
        query.model_id.as_deref(),
    );
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "route_id",
        query.route_id.as_deref(),
    );
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "status",
        query.status.as_deref(),
    );

    if let Some(search) = normalize_optional_text(query.search.as_deref()) {
        let pattern = format!("%{search}%");
        where_parts.push(
            "(provider_id LIKE ? OR model_id LIKE ? OR base_url LIKE ? OR request_preview LIKE ? OR response_preview LIKE ? OR error LIKE ?)".to_string(),
        );
        for _ in 0..6 {
            parameters.push(SqlValue::from(pattern.clone()));
        }
    }

    (
        if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        },
        parameters,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_log_records_serialize_prompt_and_completion_alias_fields() {
        let root = tempfile::tempdir().expect("temp dir");
        let repository = LocalAiProxyObservabilityRepository::new(root.path().join("logs.db"))
            .expect("create observability repository");

        repository
            .insert_request_log(LocalAiProxyRequestLogInsert {
                id: "request-1".to_string(),
                created_at: 1_744_000_000_000,
                route_id: "route-openai".to_string(),
                route_name: "OpenAI".to_string(),
                provider_id: "openai".to_string(),
                client_protocol: "openai-compatible".to_string(),
                upstream_protocol: "openai-compatible".to_string(),
                endpoint: "chat/completions".to_string(),
                status: "succeeded".to_string(),
                model_id: Some("gpt-5.4".to_string()),
                base_url: "http://127.0.0.1:21281".to_string(),
                ttft_ms: Some(42),
                total_duration_ms: 128,
                total_tokens: 12_313,
                input_tokens: 12_307,
                output_tokens: 6,
                cache_tokens: 4_096,
                request_preview: Some("hello".to_string()),
                response_preview: Some("world".to_string()),
                error: None,
                request_body: Some("{\"messages\":[]}".to_string()),
                response_body: Some("{\"usage\":{}}".to_string()),
                response_status: Some(200),
                messages: vec![LocalAiProxyLoggedMessage {
                    index: 0,
                    role: "user".to_string(),
                    content: "hello".to_string(),
                    name: None,
                    kind: None,
                }],
            })
            .expect("insert request log");

        let logs = repository
            .list_request_logs(LocalAiProxyRequestLogsQuery::default())
            .expect("list request logs");
        let record = logs.items.first().expect("request log record");
        let value = serde_json::to_value(record).expect("serialize request log");

        assert_eq!(value.pointer("/totalTokens"), Some(&json!(12_313)));
        assert_eq!(value.pointer("/inputTokens"), Some(&json!(12_307)));
        assert_eq!(value.pointer("/outputTokens"), Some(&json!(6)));
        assert_eq!(value.pointer("/cacheTokens"), Some(&json!(4_096)));
        assert_eq!(value.pointer("/promptTokens"), Some(&json!(12_307)));
        assert_eq!(value.pointer("/completionTokens"), Some(&json!(6)));
    }
}

fn build_message_logs_where_clause(
    query: &LocalAiProxyMessageLogsQuery,
) -> (String, Vec<SqlValue>) {
    let mut where_parts = Vec::new();
    let mut parameters = Vec::new();
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "provider_id",
        query.provider_id.as_deref(),
    );
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "model_id",
        query.model_id.as_deref(),
    );
    push_exact_filter(
        &mut where_parts,
        &mut parameters,
        "route_id",
        query.route_id.as_deref(),
    );

    if let Some(search) = normalize_optional_text(query.search.as_deref()) {
        let pattern = format!("%{search}%");
        where_parts.push(
            "(provider_id LIKE ? OR model_id LIKE ? OR base_url LIKE ? OR preview LIKE ? OR response_preview LIKE ? OR messages_json LIKE ?)".to_string(),
        );
        for _ in 0..6 {
            parameters.push(SqlValue::from(pattern.clone()));
        }
    }

    (
        if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        },
        parameters,
    )
}
