use crate::crypto::KeyEncryption;
use crate::error::StoreError;
use crate::id::next_runtime_id;
use crate::models::*;
use sqlx::postgres::PgPool;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

const ACCOUNTS_TABLE: &str = "local_router_upstream_accounts";
const MODEL_ROUTE_MAPPINGS_TABLE: &str = "local_router_model_route_mappings";
const CLIENT_API_KEYS_TABLE: &str = "local_router_client_api_keys";
const INVOCATIONS_TABLE: &str = "local_router_invocations";
const USAGES_TABLE: &str = "local_router_usages";
pub const DEFAULT_USER_ID: i64 = 0;

#[derive(Clone)]
pub enum DatabasePool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Clone)]
pub struct Store {
    pool: DatabasePool,
    encryption: KeyEncryption,
}

impl Store {
    pub async fn new(database_url: &str) -> Result<Self, StoreError> {
        Self::with_encryption(database_url, KeyEncryption::disabled()).await
    }

    pub async fn with_encryption(
        database_url: &str,
        encryption: KeyEncryption,
    ) -> Result<Self, StoreError> {
        // Use sdkwork-database for unified pool creation
        let pool_config = sdkwork_database_config::DatabaseConfig {
            engine: if database_url.starts_with("postgres://")
                || database_url.starts_with("postgresql://")
            {
                sdkwork_database_config::DatabaseEngine::Postgres
            } else {
                sdkwork_database_config::DatabaseEngine::Sqlite
            },
            url: database_url.to_string(),
            max_connections: if database_url.starts_with("postgres") {
                8
            } else {
                4
            },
            ..Default::default()
        };

        let pool = sdkwork_database_sqlx::create_pool_from_config(pool_config)
            .await
            .map_err(|e| StoreError::Connection(e.to_string()))?;

        match pool {
            sdkwork_database_sqlx::DatabasePool::Sqlite(sqlite_pool, _) => Ok(Self {
                pool: DatabasePool::Sqlite(sqlite_pool),
                encryption,
            }),
            sdkwork_database_sqlx::DatabasePool::Postgres(pg_pool, _) => Ok(Self {
                pool: DatabasePool::Postgres(pg_pool),
                encryption,
            }),
        }
    }

    pub async fn run_migrations(&self) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::migrate!("./migrations/sqlite")
                    .run(pool)
                    .await
                    .map_err(|e| StoreError::Migration(e.to_string()))?;
            }
            DatabasePool::Postgres(_) => {
                crate::bootstrap::bootstrap_local_router_database_from_env()
                    .await
                    .map_err(|error| StoreError::Migration(error))?;
            }
        }
        Ok(())
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self.pool, DatabasePool::Postgres(_))
    }

    // === API Key CRUD ===

    pub fn client_api_key_hash(secret: &str) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(b"sdkwork-local-router-api-key-v1:");
        hasher.update(secret.trim().as_bytes());
        let digest = hasher.finalize();
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn client_api_key_prefix(secret: &str) -> Option<String> {
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(12).collect())
        }
    }

    pub async fn insert_client_api_key(
        &self,
        client_api_key: &NewClientApiKey,
    ) -> Result<i64, StoreError> {
        let id = next_runtime_id(CLIENT_API_KEYS_TABLE)?;
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {CLIENT_API_KEYS_TABLE} (id, user_id, name, key_hash, key_prefix, enabled) VALUES (?, ?, ?, ?, ?, ?)"
                ))
                .bind(id)
                .bind(client_api_key.user_id)
                .bind(&client_api_key.name)
                .bind(&client_api_key.key_hash)
                .bind(&client_api_key.key_prefix)
                .bind(client_api_key.enabled)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(id)
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {CLIENT_API_KEYS_TABLE} (id, user_id, name, key_hash, key_prefix, enabled) VALUES ($1, $2, $3, $4, $5, $6)"
                ))
                .bind(id)
                .bind(client_api_key.user_id)
                .bind(&client_api_key.name)
                .bind(&client_api_key.key_hash)
                .bind(&client_api_key.key_prefix)
                .bind(client_api_key.enabled)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(id)
            }
        }
    }

    pub async fn list_client_api_keys_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<ClientApiKeyRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ClientApiKeyRow>(&format!(
                "SELECT * FROM {CLIENT_API_KEYS_TABLE} WHERE user_id = ? ORDER BY id DESC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ClientApiKeyRow>(&format!(
                "SELECT * FROM {CLIENT_API_KEYS_TABLE} WHERE user_id = $1 ORDER BY id DESC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }

    pub async fn get_client_api_key_for_user(
        &self,
        user_id: i64,
        id: i64,
    ) -> Result<ClientApiKeyRow, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ClientApiKeyRow>(&format!(
                "SELECT * FROM {CLIENT_API_KEYS_TABLE} WHERE user_id = ? AND id = ?"
            ))
            .bind(user_id)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound(format!("client API key {id} not found"))),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ClientApiKeyRow>(&format!(
                "SELECT * FROM {CLIENT_API_KEYS_TABLE} WHERE user_id = $1 AND id = $2"
            ))
            .bind(user_id)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound(format!("client API key {id} not found"))),
        }
    }

    pub async fn delete_client_api_key_for_user(
        &self,
        user_id: i64,
        id: i64,
    ) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {CLIENT_API_KEYS_TABLE} WHERE user_id = ? AND id = ?"
                ))
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {CLIENT_API_KEYS_TABLE} WHERE user_id = $1 AND id = $2"
                ))
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub async fn authenticate_client_api_key_secret(
        &self,
        secret: &str,
    ) -> Result<Option<ClientApiKeyRow>, StoreError> {
        let key_hash = Self::client_api_key_hash(secret);
        let row = match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ClientApiKeyRow>(&format!(
                "SELECT * FROM {CLIENT_API_KEYS_TABLE} WHERE key_hash = ? AND enabled = 1"
            ))
            .bind(&key_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?,
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ClientApiKeyRow>(&format!(
                "SELECT * FROM {CLIENT_API_KEYS_TABLE} WHERE key_hash = $1 AND enabled = TRUE"
            ))
            .bind(&key_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?,
        };
        if let Some(ref client_api_key) = row {
            self.mark_client_api_key_used(client_api_key.id).await?;
        }
        Ok(row)
    }

    async fn mark_client_api_key_used(&self, id: i64) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "UPDATE {CLIENT_API_KEYS_TABLE} SET last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?"
                ))
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "UPDATE {CLIENT_API_KEYS_TABLE} SET last_used_at = NOW() AT TIME ZONE 'UTC', updated_at = NOW() AT TIME ZONE 'UTC' WHERE id = $1"
                ))
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    // === Account CRUD ===

    pub async fn list_accounts(&self) -> Result<Vec<AccountRow>, StoreError> {
        self.list_accounts_for_user(DEFAULT_USER_ID).await
    }

    pub async fn list_accounts_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<AccountRow>, StoreError> {
        let mut rows = match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, AccountRow>(&format!(
                "SELECT * FROM {ACCOUNTS_TABLE} WHERE user_id = ? ORDER BY priority ASC, name ASC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?,
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AccountRow>(&format!(
                "SELECT * FROM {ACCOUNTS_TABLE} WHERE user_id = $1 ORDER BY priority ASC, name ASC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?,
        };
        for row in &mut rows {
            row.upstream_api_key = self.encryption.decrypt(&row.upstream_api_key);
        }
        Ok(rows)
    }

    pub async fn get_account(&self, id: i64) -> Result<AccountRow, StoreError> {
        self.get_account_for_user(DEFAULT_USER_ID, id).await
    }

    pub async fn get_account_for_user(
        &self,
        user_id: i64,
        id: i64,
    ) -> Result<AccountRow, StoreError> {
        let mut row = match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, AccountRow>(&format!(
                "SELECT * FROM {ACCOUNTS_TABLE} WHERE user_id = ? AND id = ?"
            ))
            .bind(user_id)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound(format!("account {} not found", id)))?,
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AccountRow>(&format!(
                "SELECT * FROM {ACCOUNTS_TABLE} WHERE user_id = $1 AND id = $2"
            ))
            .bind(user_id)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound(format!("account {} not found", id)))?,
        };
        row.upstream_api_key = self.encryption.decrypt(&row.upstream_api_key);
        Ok(row)
    }

    pub async fn insert_account(&self, account: &NewAccount) -> Result<i64, StoreError> {
        let account_id = next_runtime_id(ACCOUNTS_TABLE)?;
        let encrypted_upstream_api_key = self.encryption.encrypt(&account.upstream_api_key);
        let models_json =
            serde_json::to_string(&account.models).unwrap_or_else(|_| "[]".to_owned());
        let headers_json =
            serde_json::to_string(&account.default_headers).unwrap_or_else(|_| "{}".to_owned());
        let route_mappings_json = serde_json::to_string(&account.model_route_mappings)
            .unwrap_or_else(|_| "{}".to_owned());

        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {ACCOUNTS_TABLE} (id, user_id, name, provider, base_url, upstream_api_key, upstream_auth_scheme, models, priority, timeout_secs, max_retries, retry_delay_ms, anthropic_version, default_headers, model_route_mappings, enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                ))
                .bind(account_id)
                .bind(account.user_id).bind(&account.name).bind(&account.provider).bind(&account.base_url).bind(&encrypted_upstream_api_key)
                .bind(&account.upstream_auth_scheme)
                .bind(&models_json).bind(account.priority).bind(account.timeout_secs)
                .bind(account.max_retries).bind(account.retry_delay_ms)
                .bind(&account.anthropic_version).bind(&headers_json).bind(&route_mappings_json)
                .bind(account.enabled)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
                self.replace_model_route_mappings_for_account_from_map(
                    account.user_id,
                    account_id,
                    &account.name,
                    &account.model_route_mappings,
                )
                .await?;
                Ok(account_id)
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {ACCOUNTS_TABLE} (id, user_id, name, provider, base_url, upstream_api_key, upstream_auth_scheme, models, priority, timeout_secs, max_retries, retry_delay_ms, anthropic_version, default_headers, model_route_mappings, enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"
                ))
                .bind(account_id)
                .bind(account.user_id).bind(&account.name).bind(&account.provider).bind(&account.base_url).bind(&encrypted_upstream_api_key)
                .bind(&account.upstream_auth_scheme)
                .bind(&models_json).bind(account.priority).bind(account.timeout_secs)
                .bind(account.max_retries).bind(account.retry_delay_ms)
                .bind(&account.anthropic_version).bind(&headers_json).bind(&route_mappings_json)
                .bind(account.enabled)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
                self.replace_model_route_mappings_for_account_from_map(
                    account.user_id,
                    account_id,
                    &account.name,
                    &account.model_route_mappings,
                )
                .await?;
                Ok(account_id)
            }
        }
    }

    pub async fn update_account(&self, id: i64, account: &NewAccount) -> Result<(), StoreError> {
        self.update_account_for_user(account.user_id, id, account)
            .await
    }

    pub async fn update_account_for_user(
        &self,
        user_id: i64,
        id: i64,
        account: &NewAccount,
    ) -> Result<(), StoreError> {
        self.update_account_for_user_with_route_mapping_sync(user_id, id, account, true)
            .await
    }

    pub async fn update_account_for_user_without_route_mapping_sync(
        &self,
        user_id: i64,
        id: i64,
        account: &NewAccount,
    ) -> Result<(), StoreError> {
        self.update_account_for_user_with_route_mapping_sync(user_id, id, account, false)
            .await
    }

    async fn update_account_for_user_with_route_mapping_sync(
        &self,
        user_id: i64,
        id: i64,
        account: &NewAccount,
        sync_model_route_mappings: bool,
    ) -> Result<(), StoreError> {
        let encrypted_upstream_api_key = self.encryption.encrypt(&account.upstream_api_key);
        let models_json =
            serde_json::to_string(&account.models).unwrap_or_else(|_| "[]".to_owned());
        let headers_json =
            serde_json::to_string(&account.default_headers).unwrap_or_else(|_| "{}".to_owned());
        let route_mappings_json = serde_json::to_string(&account.model_route_mappings)
            .unwrap_or_else(|_| "{}".to_owned());

        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "UPDATE {ACCOUNTS_TABLE} SET name=?, provider=?, base_url=?, upstream_api_key=?, upstream_auth_scheme=?, models=?, priority=?, timeout_secs=?, max_retries=?, retry_delay_ms=?, anthropic_version=?, default_headers=?, model_route_mappings=?, enabled=?, updated_at=datetime('now') WHERE user_id=? AND id=?"
                ))
                .bind(&account.name).bind(&account.provider).bind(&account.base_url).bind(&encrypted_upstream_api_key)
                .bind(&account.upstream_auth_scheme)
                .bind(&models_json).bind(account.priority).bind(account.timeout_secs)
                .bind(account.max_retries).bind(account.retry_delay_ms)
                .bind(&account.anthropic_version).bind(&headers_json).bind(&route_mappings_json)
                .bind(account.enabled).bind(user_id).bind(id)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "UPDATE {ACCOUNTS_TABLE} SET name=$1, provider=$2, base_url=$3, upstream_api_key=$4, upstream_auth_scheme=$5, models=$6, priority=$7, timeout_secs=$8, max_retries=$9, retry_delay_ms=$10, anthropic_version=$11, default_headers=$12, model_route_mappings=$13, enabled=$14, updated_at=NOW() WHERE user_id=$15 AND id=$16"
                ))
                .bind(&account.name).bind(&account.provider).bind(&account.base_url).bind(&encrypted_upstream_api_key)
                .bind(&account.upstream_auth_scheme)
                .bind(&models_json).bind(account.priority).bind(account.timeout_secs)
                .bind(account.max_retries).bind(account.retry_delay_ms)
                .bind(&account.anthropic_version).bind(&headers_json).bind(&route_mappings_json)
                .bind(account.enabled).bind(user_id).bind(id)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        if sync_model_route_mappings {
            self.replace_model_route_mappings_for_account_from_map(
                user_id,
                id,
                &account.name,
                &account.model_route_mappings,
            )
            .await?;
        } else {
            self.update_model_route_mapping_account_name_for_account(user_id, id, &account.name)
                .await?;
        }
        Ok(())
    }

    /// Insert-or-update by name. Used by the config file hot-reload path.
    pub async fn upsert_account(&self, account: &NewAccount) -> Result<(), StoreError> {
        if let Some(existing) = self
            .get_account_by_name_for_user(account.user_id, &account.name)
            .await?
        {
            self.update_account(existing.id, account).await
        } else {
            self.insert_account(account).await.map(|_| ())
        }
    }

    pub async fn get_account_by_name(&self, name: &str) -> Result<Option<AccountRow>, StoreError> {
        self.get_account_by_name_for_user(DEFAULT_USER_ID, name)
            .await
    }

    pub async fn get_account_by_name_for_user(
        &self,
        user_id: i64,
        name: &str,
    ) -> Result<Option<AccountRow>, StoreError> {
        let mut row = match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, AccountRow>(&format!(
                "SELECT * FROM {ACCOUNTS_TABLE} WHERE user_id = ? AND name = ?"
            ))
            .bind(user_id)
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?,
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AccountRow>(&format!(
                "SELECT * FROM {ACCOUNTS_TABLE} WHERE user_id = $1 AND name = $2"
            ))
            .bind(user_id)
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?,
        };
        if let Some(ref mut r) = row {
            r.upstream_api_key = self.encryption.decrypt(&r.upstream_api_key);
        }
        Ok(row)
    }

    pub async fn delete_account(&self, id: i64) -> Result<(), StoreError> {
        self.delete_account_for_user(DEFAULT_USER_ID, id).await
    }

    pub async fn delete_account_for_user(&self, user_id: i64, id: i64) -> Result<(), StoreError> {
        self.delete_model_route_mappings_for_account(user_id, id)
            .await?;
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {ACCOUNTS_TABLE} WHERE user_id = ? AND id = ?"
                ))
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {ACCOUNTS_TABLE} WHERE user_id = $1 AND id = $2"
                ))
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    // === Model Route Mapping CRUD ===

    pub async fn list_model_route_mappings_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<ModelRouteMappingRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = ? ORDER BY account_name ASC, client_model ASC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = $1 ORDER BY account_name ASC, client_model ASC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }

    pub async fn list_enabled_model_route_mappings_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<ModelRouteMappingRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = ? AND enabled = 1 ORDER BY account_name ASC, client_model ASC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = $1 AND enabled = TRUE ORDER BY account_name ASC, client_model ASC"
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }

    pub async fn list_model_route_mappings_for_account(
        &self,
        user_id: i64,
        account_id: i64,
    ) -> Result<Vec<ModelRouteMappingRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = ? AND account_id = ? ORDER BY client_model ASC"
            ))
            .bind(user_id)
            .bind(account_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = $1 AND account_id = $2 ORDER BY client_model ASC"
            ))
            .bind(user_id)
            .bind(account_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }

    pub async fn upsert_model_route_mapping(
        &self,
        route_mapping: &NewModelRouteMapping,
    ) -> Result<i64, StoreError> {
        let id = next_runtime_id(MODEL_ROUTE_MAPPINGS_TABLE)?;
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {MODEL_ROUTE_MAPPINGS_TABLE} (id, user_id, account_id, account_name, client_model, upstream_model, enabled, notes) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(user_id, account_id, client_model) DO UPDATE SET account_name=excluded.account_name, upstream_model=excluded.upstream_model, enabled=excluded.enabled, notes=excluded.notes, version=version + 1, updated_at=strftime('%Y-%m-%dT%H:%M:%SZ', 'now')"
                ))
                .bind(id)
                .bind(route_mapping.user_id)
                .bind(route_mapping.account_id)
                .bind(&route_mapping.account_name)
                .bind(&route_mapping.client_model)
                .bind(&route_mapping.upstream_model)
                .bind(route_mapping.enabled)
                .bind(&route_mapping.notes)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                self.get_model_route_mapping_by_client_model(
                    route_mapping.user_id,
                    route_mapping.account_id,
                    &route_mapping.client_model,
                )
                .await
                .map(|row| row.id)
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {MODEL_ROUTE_MAPPINGS_TABLE} (id, user_id, account_id, account_name, client_model, upstream_model, enabled, notes) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT(user_id, account_id, client_model) DO UPDATE SET account_name=excluded.account_name, upstream_model=excluded.upstream_model, enabled=excluded.enabled, notes=excluded.notes, version={MODEL_ROUTE_MAPPINGS_TABLE}.version + 1, updated_at=NOW()"
                ))
                .bind(id)
                .bind(route_mapping.user_id)
                .bind(route_mapping.account_id)
                .bind(&route_mapping.account_name)
                .bind(&route_mapping.client_model)
                .bind(&route_mapping.upstream_model)
                .bind(route_mapping.enabled)
                .bind(&route_mapping.notes)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                self.get_model_route_mapping_by_client_model(
                    route_mapping.user_id,
                    route_mapping.account_id,
                    &route_mapping.client_model,
                )
                .await
                .map(|row| row.id)
            }
        }
    }

    pub async fn replace_model_route_mappings_for_account_from_map(
        &self,
        user_id: i64,
        account_id: i64,
        account_name: &str,
        route_mappings: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), StoreError> {
        self.delete_model_route_mappings_for_account(user_id, account_id)
            .await?;
        for (client_model, upstream_model) in route_mappings {
            if client_model.trim().is_empty() || upstream_model.trim().is_empty() {
                continue;
            }
            self.upsert_model_route_mapping(&NewModelRouteMapping {
                user_id,
                account_id,
                account_name: account_name.to_owned(),
                client_model: client_model.trim().to_owned(),
                upstream_model: upstream_model.trim().to_owned(),
                enabled: true,
                notes: Some("synced from account model_route_mappings".to_owned()),
            })
            .await?;
        }
        Ok(())
    }

    pub async fn delete_model_route_mapping_for_user(
        &self,
        user_id: i64,
        id: i64,
    ) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = ? AND id = ?"
                ))
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = $1 AND id = $2"
                ))
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    async fn delete_model_route_mappings_for_account(
        &self,
        user_id: i64,
        account_id: i64,
    ) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = ? AND account_id = ?"
                ))
                .bind(user_id)
                .bind(account_id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "DELETE FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = $1 AND account_id = $2"
                ))
                .bind(user_id)
                .bind(account_id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    async fn update_model_route_mapping_account_name_for_account(
        &self,
        user_id: i64,
        account_id: i64,
        account_name: &str,
    ) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "UPDATE {MODEL_ROUTE_MAPPINGS_TABLE} SET account_name = ?, version = version + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE user_id = ? AND account_id = ? AND account_name <> ?"
                ))
                .bind(account_name)
                .bind(user_id)
                .bind(account_id)
                .bind(account_name)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "UPDATE {MODEL_ROUTE_MAPPINGS_TABLE} SET account_name = $1, version = version + 1, updated_at = NOW() WHERE user_id = $2 AND account_id = $3 AND account_name <> $1"
                ))
                .bind(account_name)
                .bind(user_id)
                .bind(account_id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    async fn get_model_route_mapping_by_client_model(
        &self,
        user_id: i64,
        account_id: i64,
        client_model: &str,
    ) -> Result<ModelRouteMappingRow, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = ? AND account_id = ? AND client_model = ?"
            ))
            .bind(user_id)
            .bind(account_id)
            .bind(client_model)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound("model route mapping not found".to_owned())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, ModelRouteMappingRow>(&format!(
                "SELECT * FROM {MODEL_ROUTE_MAPPINGS_TABLE} WHERE user_id = $1 AND account_id = $2 AND client_model = $3"
            ))
            .bind(user_id)
            .bind(account_id)
            .bind(client_model)
            .fetch_optional(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?
            .ok_or_else(|| StoreError::NotFound("model route mapping not found".to_owned())),
        }
    }

    pub async fn toggle_account(&self, id: i64, enabled: bool) -> Result<(), StoreError> {
        self.toggle_account_for_user(DEFAULT_USER_ID, id, enabled)
            .await
    }

    pub async fn toggle_account_for_user(
        &self,
        user_id: i64,
        id: i64,
        enabled: bool,
    ) -> Result<(), StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "UPDATE {ACCOUNTS_TABLE} SET enabled = ?, updated_at = datetime('now') WHERE user_id = ? AND id = ?",
                ))
                .bind(enabled)
                .bind(user_id)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "UPDATE {ACCOUNTS_TABLE} SET enabled = $1, updated_at = NOW() WHERE user_id = $2 AND id = $3"
                ))
                    .bind(enabled)
                    .bind(user_id)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    // === Invocation CRUD ===

    pub async fn insert_invocation(&self, inv: &NewInvocation) -> Result<(), StoreError> {
        let id = next_runtime_id(INVOCATIONS_TABLE)?;
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {INVOCATIONS_TABLE} (id, user_id, request_id, account_name, protocol, method, path, query, model, status, status_code, latency_ms, error_message, request_body, response_body, request_body_size, response_body_size, upstream_protocol, upstream_path, client_api, request_surface, target_surface, plugin_policy, plugin_id, model_vendor, metadata, streaming, attempt_count) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                ))
                .bind(id)
                .bind(inv.user_id).bind(&inv.request_id).bind(&inv.account_name).bind(&inv.protocol)
                .bind(&inv.method).bind(&inv.path).bind(&inv.query).bind(&inv.model)
                .bind(&inv.status).bind(inv.status_code).bind(inv.latency_ms).bind(&inv.error_message)
                .bind(&inv.request_body).bind(&inv.response_body)
                .bind(inv.request_body_size).bind(inv.response_body_size)
                .bind(&inv.upstream_protocol).bind(&inv.upstream_path).bind(&inv.client_api)
                .bind(&inv.request_surface).bind(&inv.target_surface).bind(&inv.plugin_policy)
                .bind(&inv.plugin_id).bind(&inv.model_vendor).bind(inv.metadata.as_deref().unwrap_or("{}"))
                .bind(inv.streaming).bind(inv.attempt_count)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {INVOCATIONS_TABLE} (id, user_id, request_id, account_name, protocol, method, path, query, model, status, status_code, latency_ms, error_message, request_body, response_body, request_body_size, response_body_size, upstream_protocol, upstream_path, client_api, request_surface, target_surface, plugin_policy, plugin_id, model_vendor, metadata, streaming, attempt_count) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26::jsonb, $27, $28)"
                ))
                .bind(id)
                .bind(inv.user_id).bind(&inv.request_id).bind(&inv.account_name).bind(&inv.protocol)
                .bind(&inv.method).bind(&inv.path).bind(&inv.query).bind(&inv.model)
                .bind(&inv.status).bind(inv.status_code).bind(inv.latency_ms).bind(&inv.error_message)
                .bind(&inv.request_body).bind(&inv.response_body)
                .bind(inv.request_body_size).bind(inv.response_body_size)
                .bind(&inv.upstream_protocol).bind(&inv.upstream_path).bind(&inv.client_api)
                .bind(&inv.request_surface).bind(&inv.target_surface).bind(&inv.plugin_policy)
                .bind(&inv.plugin_id).bind(&inv.model_vendor).bind(inv.metadata.as_deref().unwrap_or("{}"))
                .bind(inv.streaming).bind(inv.attempt_count)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub async fn list_invocations(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<InvocationRow>, StoreError> {
        self.list_invocations_for_user(DEFAULT_USER_ID, limit, offset)
            .await
    }

    pub async fn list_invocations_for_user(
        &self,
        user_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<InvocationRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, InvocationRow>(&format!(
                "SELECT * FROM {INVOCATIONS_TABLE} WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
            ))
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, InvocationRow>(&format!(
                "SELECT id, user_id, request_id, account_name, protocol, method, path, query, model, status, status_code, latency_ms, error_message, request_body, response_body, request_body_size, response_body_size, upstream_protocol, upstream_path, client_api, request_surface, target_surface, plugin_policy, plugin_id, model_vendor, metadata::text AS metadata, streaming, attempt_count, created_at::text AS created_at FROM {INVOCATIONS_TABLE} WHERE user_id = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
            ))
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }

    // === Usage CRUD ===

    pub async fn insert_usage(&self, usage: &NewUsage) -> Result<(), StoreError> {
        let id = next_runtime_id(USAGES_TABLE)?;
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {USAGES_TABLE} (id, user_id, request_id, model, prompt_tokens, completion_tokens, total_tokens) VALUES (?, ?, ?, ?, ?, ?, ?)"
                ))
                .bind(id)
                .bind(usage.user_id).bind(&usage.request_id).bind(&usage.model)
                .bind(usage.prompt_tokens).bind(usage.completion_tokens).bind(usage.total_tokens)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(&format!(
                    "INSERT INTO {USAGES_TABLE} (id, user_id, request_id, model, prompt_tokens, completion_tokens, total_tokens) VALUES ($1, $2, $3, $4, $5, $6, $7)"
                ))
                .bind(id)
                .bind(usage.user_id).bind(&usage.request_id).bind(&usage.model)
                .bind(usage.prompt_tokens).bind(usage.completion_tokens).bind(usage.total_tokens)
                .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub async fn list_usages(&self, limit: i64, offset: i64) -> Result<Vec<UsageRow>, StoreError> {
        self.list_usages_for_user(DEFAULT_USER_ID, limit, offset)
            .await
    }

    pub async fn list_usages_for_user(
        &self,
        user_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UsageRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, UsageRow>(&format!(
                "SELECT * FROM {USAGES_TABLE} WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
            ))
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, UsageRow>(&format!(
                "SELECT * FROM {USAGES_TABLE} WHERE user_id = $1 ORDER BY id DESC LIMIT $2 OFFSET $3"
            ))
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }

    // === Legacy compatibility ===

    pub async fn insert_request_log(
        &self,
        request_id: &str,
        protocol: &str,
        method: &str,
        path: &str,
        model: Option<&str>,
        upstream_name: Option<&str>,
        status_code: Option<i32>,
        _request_body_size: Option<i64>,
        _response_body_size: Option<i64>,
        latency_ms: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<(), StoreError> {
        let inv = NewInvocation {
            user_id: DEFAULT_USER_ID,
            request_id: request_id.to_owned(),
            account_name: upstream_name.map(String::from),
            protocol: protocol.to_owned(),
            method: method.to_owned(),
            path: path.to_owned(),
            query: None,
            model: model.map(String::from),
            status: if error_message.is_some() {
                "error".to_owned()
            } else if status_code.is_some() {
                "completed".to_owned()
            } else {
                "pending".to_owned()
            },
            status_code,
            latency_ms,
            error_message: error_message.map(String::from),
            request_body: None,
            response_body: None,
            request_body_size: None,
            response_body_size: None,
            upstream_protocol: None,
            upstream_path: None,
            client_api: None,
            request_surface: None,
            target_surface: None,
            plugin_policy: None,
            plugin_id: None,
            model_vendor: None,
            metadata: None,
            streaming: false,
            attempt_count: 1,
        };
        self.insert_invocation(&inv).await
    }

    pub async fn recent_request_logs(&self, limit: i64) -> Result<Vec<InvocationRow>, StoreError> {
        self.list_invocations(limit, 0).await
    }

    // === Cleanup ===

    pub async fn cleanup_invocations(&self, retention_days: i64) -> Result<u64, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query(&format!(
                    "DELETE FROM {INVOCATIONS_TABLE} WHERE created_at < datetime('now', ? || ' days')",
                ))
                .bind(format!("-{}", retention_days))
                .execute(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(result.rows_affected())
            }
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(&format!(
                    "DELETE FROM {INVOCATIONS_TABLE} WHERE created_at < NOW() AT TIME ZONE 'UTC' - $1 * INTERVAL '1 day'"
                )).bind(retention_days)
                    .execute(pool).await.map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(result.rows_affected())
            }
        }
    }

    // === Aggregate Queries ===

    pub async fn count_invocations(&self) -> Result<i64, StoreError> {
        self.count_invocations_for_user(DEFAULT_USER_ID).await
    }

    pub async fn count_invocations_for_user(&self, user_id: i64) -> Result<i64, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(&format!(
                    "SELECT COUNT(*) as count FROM {INVOCATIONS_TABLE} WHERE user_id = ?"
                ))
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(row.get::<i64, _>("count"))
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(&format!(
                    "SELECT COUNT(*) as count FROM {INVOCATIONS_TABLE} WHERE user_id = $1"
                ))
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(row.get::<i64, _>("count"))
            }
        }
    }

    pub async fn count_accounts(&self) -> Result<i64, StoreError> {
        self.count_accounts_for_user(DEFAULT_USER_ID).await
    }

    pub async fn count_accounts_for_user(&self, user_id: i64) -> Result<i64, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(&format!(
                    "SELECT COUNT(*) as count FROM {ACCOUNTS_TABLE} WHERE user_id = ?"
                ))
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(row.get::<i64, _>("count"))
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(&format!(
                    "SELECT COUNT(*) as count FROM {ACCOUNTS_TABLE} WHERE user_id = $1"
                ))
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(row.get::<i64, _>("count"))
            }
        }
    }

    pub async fn usage_totals(&self) -> Result<UsageTotals, StoreError> {
        self.usage_totals_for_user(DEFAULT_USER_ID).await
    }

    pub async fn usage_totals_for_user(&self, user_id: i64) -> Result<UsageTotals, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(&format!(
                    "SELECT COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
                     COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
                     COALESCE(SUM(total_tokens), 0) as total_tokens \
                     FROM {USAGES_TABLE} WHERE user_id = ?",
                ))
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(UsageTotals {
                    prompt_tokens: row.get::<i64, _>("prompt_tokens"),
                    completion_tokens: row.get::<i64, _>("completion_tokens"),
                    total_tokens: row.get::<i64, _>("total_tokens"),
                })
            }
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(&format!(
                    "SELECT COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
                     COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
                     COALESCE(SUM(total_tokens), 0) as total_tokens \
                     FROM {USAGES_TABLE} WHERE user_id = $1",
                ))
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
                Ok(UsageTotals {
                    prompt_tokens: row.get::<i64, _>("prompt_tokens"),
                    completion_tokens: row.get::<i64, _>("completion_tokens"),
                    total_tokens: row.get::<i64, _>("total_tokens"),
                })
            }
        }
    }

    pub async fn usage_by_model(&self) -> Result<Vec<UsageByModelRow>, StoreError> {
        self.usage_by_model_for_user(DEFAULT_USER_ID).await
    }

    pub async fn usage_by_model_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<UsageByModelRow>, StoreError> {
        match &self.pool {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, UsageByModelRow>(&format!(
                "SELECT user_id, model, COUNT(*) as request_count, \
                     COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
                     COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
                     COALESCE(SUM(total_tokens), 0) as total_tokens \
                     FROM {USAGES_TABLE} WHERE user_id = ? GROUP BY user_id, model ORDER BY total_tokens DESC",
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, UsageByModelRow>(&format!(
                "SELECT user_id, model, COUNT(*) as request_count, \
                     COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
                     COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
                     COALESCE(SUM(total_tokens), 0) as total_tokens \
                     FROM {USAGES_TABLE} WHERE user_id = $1 GROUP BY user_id, model ORDER BY total_tokens DESC",
            ))
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| StoreError::Query(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn temp_sqlite_url(name: &str) -> (String, PathBuf) {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("sdkwork-local-router-{name}-{nanos}.db"));
        let url = format!("sqlite:{}?mode=rwc", path.to_string_lossy());
        (url, path)
    }

    fn assert_runtime_snowflake_id(id: i64) {
        assert!(
            id > 1_000_000_000_000,
            "runtime id {id} must be generated by the Snowflake id generator, not the database"
        );
    }

    #[tokio::test]
    async fn migrations_create_only_local_router_owned_tables() {
        let (url, path) = temp_sqlite_url("prefixed-tables");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let DatabasePool::Sqlite(pool) = &store.pool else {
            unreachable!("test uses sqlite");
        };
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE '_sqlx_%' ORDER BY name",
        )
        .fetch_all(pool)
        .await
        .unwrap();

        assert!(tables.contains(&"local_router_upstream_accounts".to_owned()));
        assert!(tables.contains(&"local_router_model_route_mappings".to_owned()));
        assert!(tables.contains(&"local_router_client_api_keys".to_owned()));
        assert!(!tables.contains(&"local_router_accounts".to_owned()));
        assert!(!tables.contains(&"local_router_api_keys".to_owned()));
        assert!(tables.contains(&"local_router_invocations".to_owned()));
        assert!(tables.contains(&"local_router_usages".to_owned()));
        assert!(!tables.contains(&"local_router_users".to_owned()));
        assert!(!tables.contains(&"accounts".to_owned()));
        assert!(!tables.contains(&"invocations".to_owned()));
        assert!(!tables.contains(&"usages".to_owned()));
        assert!(tables
            .iter()
            .filter(|name| name.as_str() != "sqlite_sequence")
            .all(|name| name.starts_with("local_router_")));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migration_sources_do_not_create_unprefixed_router_tables() {
        let migration_sources = [
            include_str!("../migrations/sqlite/20260522000001_init.sql"),
            include_str!("../migrations/postgres/20260522000001_init.sql"),
        ];

        for source in migration_sources {
            let normalized = source.to_ascii_lowercase();
            assert!(!normalized.contains("create table if not exists accounts"));
            assert!(!normalized.contains("create table if not exists invocations"));
            assert!(!normalized.contains("create table if not exists usages"));
            assert!(!normalized.contains("api_key             "));
        }
    }

    #[test]
    fn migration_sources_do_not_allocate_runtime_ids_in_database() {
        let migration_sources = [
            include_str!("../migrations/sqlite/20260522000001_init.sql"),
            include_str!("../migrations/sqlite/20260604000004_add_model_route_mappings_table.sql"),
            include_str!("../migrations/postgres/20260522000001_init.sql"),
            include_str!(
                "../migrations/postgres/20260604000004_add_model_route_mappings_table.sql"
            ),
        ];

        for source in migration_sources {
            let normalized = source.to_ascii_lowercase();
            assert!(!normalized.contains("bigserial"));
            assert!(!normalized.contains(&format!("auto{}", "increment")));
        }
    }

    #[tokio::test]
    async fn runtime_business_inserts_use_explicit_snowflake_ids() {
        let (url, path) = temp_sqlite_url("snowflake-runtime-ids");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let key_id = store
            .insert_client_api_key(&NewClientApiKey {
                user_id: 31,
                name: "runtime key".to_owned(),
                key_hash: Store::client_api_key_hash("sk-runtime-id"),
                key_prefix: Store::client_api_key_prefix("sk-runtime-id"),
                enabled: true,
            })
            .await
            .unwrap();
        assert_runtime_snowflake_id(key_id);

        let account_id = store
            .insert_account(&NewAccount {
                user_id: 31,
                name: "runtime-account".to_owned(),
                provider: "openai".to_owned(),
                base_url: "https://runtime.example/v1".to_owned(),
                upstream_api_key: "sk-test".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["gpt-runtime".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: BTreeMap::new(),
                model_route_mappings: BTreeMap::new(),
                enabled: true,
            })
            .await
            .unwrap();
        assert_runtime_snowflake_id(account_id);

        let route_mapping_id = store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 31,
                account_id,
                account_name: "runtime-account".to_owned(),
                client_model: "gpt-5.5".to_owned(),
                upstream_model: "gpt-runtime".to_owned(),
                enabled: true,
                notes: None,
            })
            .await
            .unwrap();
        assert_runtime_snowflake_id(route_mapping_id);

        store
            .insert_invocation(&NewInvocation {
                user_id: 31,
                request_id: "runtime-request".to_owned(),
                account_name: Some("runtime-account".to_owned()),
                protocol: "openai".to_owned(),
                method: "POST".to_owned(),
                path: "/v1/responses".to_owned(),
                query: None,
                model: Some("gpt-5.5".to_owned()),
                status: "completed".to_owned(),
                status_code: Some(200),
                latency_ms: Some(12),
                error_message: None,
                request_body: None,
                response_body: None,
                request_body_size: Some(2),
                response_body_size: Some(2),
                upstream_protocol: Some("openai".to_owned()),
                upstream_path: Some("/responses".to_owned()),
                client_api: Some("codex".to_owned()),
                request_surface: Some("OPENAI_RESPONSES".to_owned()),
                target_surface: Some("OPENAI_RESPONSES".to_owned()),
                plugin_policy: Some("auto".to_owned()),
                plugin_id: None,
                model_vendor: Some("openai".to_owned()),
                metadata: None,
                streaming: false,
                attempt_count: 1,
            })
            .await
            .unwrap();
        let invocation = store
            .list_invocations_for_user(31, 10, 0)
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("invocation row must be stored");
        assert_runtime_snowflake_id(invocation.id);

        store
            .insert_usage(&NewUsage {
                user_id: 31,
                request_id: "runtime-request".to_owned(),
                model: Some("gpt-5.5".to_owned()),
                prompt_tokens: Some(8),
                completion_tokens: Some(5),
                total_tokens: Some(13),
            })
            .await
            .unwrap();
        let usage = store
            .list_usages_for_user(31, 10, 0)
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("usage row must be stored");
        assert_runtime_snowflake_id(usage.id);

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn client_api_key_authentication_resolves_user_id_from_database_record() {
        let (url, path) = temp_sqlite_url("api-key-user-id");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let secret = "sk-local-user-42";
        let key_id = store
            .insert_client_api_key(&NewClientApiKey {
                user_id: 42,
                name: "user 42 runtime key".to_owned(),
                key_hash: Store::client_api_key_hash(secret),
                key_prefix: Store::client_api_key_prefix(secret),
                enabled: true,
            })
            .await
            .unwrap();
        store
            .insert_client_api_key(&NewClientApiKey {
                user_id: 7,
                name: "disabled key".to_owned(),
                key_hash: Store::client_api_key_hash("sk-disabled"),
                key_prefix: Store::client_api_key_prefix("sk-disabled"),
                enabled: false,
            })
            .await
            .unwrap();

        let authenticated = store
            .authenticate_client_api_key_secret(secret)
            .await
            .unwrap()
            .expect("client API key should authenticate");

        assert_eq!(authenticated.id, key_id);
        assert_eq!(authenticated.user_id, 42);
        assert_eq!(authenticated.key_prefix.as_deref(), Some("sk-local-use"));
        assert!(store
            .authenticate_client_api_key_secret("sk-disabled")
            .await
            .unwrap()
            .is_none());
        assert!(store
            .authenticate_client_api_key_secret("sk-missing")
            .await
            .unwrap()
            .is_none());

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn invocation_roundtrip_records_router_plugin_and_attempt_metadata() {
        let (url, path) = temp_sqlite_url("invocation-metadata");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let account = NewAccount {
            user_id: DEFAULT_USER_ID,
            name: "openai-main".to_owned(),
            provider: "openai".to_owned(),
            base_url: "https://api.openai.example/v1".to_owned(),
            upstream_api_key: "sk-test".to_owned(),
            upstream_auth_scheme: Some("bearer".to_owned()),
            models: vec!["gpt-5".to_owned()],
            priority: 10,
            timeout_secs: 120,
            max_retries: 1,
            retry_delay_ms: 500,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            model_route_mappings: BTreeMap::new(),
            enabled: true,
        };
        store.insert_account(&account).await.unwrap();
        let stored_account = store
            .get_account_by_name("openai-main")
            .await
            .unwrap()
            .expect("stored account");
        assert_eq!(
            stored_account.upstream_auth_scheme.as_deref(),
            Some("bearer")
        );

        store
            .insert_invocation(&NewInvocation {
                user_id: DEFAULT_USER_ID,
                request_id: "req-1".to_owned(),
                account_name: Some("openai-main".to_owned()),
                protocol: "openai".to_owned(),
                method: "POST".to_owned(),
                path: "/v1/responses".to_owned(),
                query: Some("trace=1".to_owned()),
                model: Some("gpt-5".to_owned()),
                status: "completed".to_owned(),
                status_code: Some(200),
                latency_ms: Some(123),
                error_message: None,
                request_body: Some("{\"model\":\"gpt-5\"}".to_owned()),
                response_body: Some("{\"id\":\"resp_1\"}".to_owned()),
                request_body_size: Some(17),
                response_body_size: Some(15),
                upstream_protocol: Some("openai".to_owned()),
                upstream_path: Some("/responses".to_owned()),
                client_api: Some("codex".to_owned()),
                request_surface: Some("OPENAI_RESPONSES".to_owned()),
                target_surface: Some("OPENAI_RESPONSES".to_owned()),
                plugin_policy: Some("auto".to_owned()),
                plugin_id: None,
                model_vendor: Some("openai".to_owned()),
                metadata: Some(r#"{"audit.user_id":"0","route_candidate_count":"1"}"#.to_owned()),
                streaming: false,
                attempt_count: 2,
            })
            .await
            .unwrap();

        let rows = store.list_invocations(10, 0).await.unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.request_id, "req-1");
        assert_eq!(row.query.as_deref(), Some("trace=1"));
        assert_eq!(row.status, "completed");
        assert_eq!(row.upstream_path.as_deref(), Some("/responses"));
        assert_eq!(row.client_api.as_deref(), Some("codex"));
        assert_eq!(row.request_surface.as_deref(), Some("OPENAI_RESPONSES"));
        assert_eq!(row.target_surface.as_deref(), Some("OPENAI_RESPONSES"));
        assert_eq!(row.plugin_policy.as_deref(), Some("auto"));
        assert_eq!(row.model_vendor.as_deref(), Some("openai"));
        assert_eq!(
            row.metadata.as_deref(),
            Some(r#"{"audit.user_id":"0","route_candidate_count":"1"}"#)
        );
        assert!(!row.streaming);
        assert_eq!(row.attempt_count, 2);
        assert_eq!(row.request_body_size, Some(17));
        assert_eq!(row.response_body_size, Some(15));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn accounts_invocations_and_usage_are_isolated_by_user() {
        let (url, path) = temp_sqlite_url("user-isolation");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        for user_id in [1_i64, 2_i64] {
            store
                .insert_account(&NewAccount {
                    user_id,
                    name: "shared-name".to_owned(),
                    provider: "openai".to_owned(),
                    base_url: format!("https://user{user_id}.example/v1"),
                    upstream_api_key: "sk-test".to_owned(),
                    upstream_auth_scheme: None,
                    models: vec![format!("user{user_id}-model")],
                    priority: 10,
                    timeout_secs: 120,
                    max_retries: 0,
                    retry_delay_ms: 500,
                    anthropic_version: None,
                    default_headers: BTreeMap::new(),
                    model_route_mappings: BTreeMap::new(),
                    enabled: true,
                })
                .await
                .unwrap();
        }

        let alice_accounts = store.list_accounts_for_user(1).await.unwrap();
        let bob_accounts = store.list_accounts_for_user(2).await.unwrap();
        assert_eq!(alice_accounts.len(), 1);
        assert_eq!(bob_accounts.len(), 1);
        assert_eq!(alice_accounts[0].name, "shared-name");
        assert_eq!(bob_accounts[0].name, "shared-name");
        assert_ne!(alice_accounts[0].base_url, bob_accounts[0].base_url);

        store
            .insert_invocation(&NewInvocation {
                user_id: 1,
                request_id: "alice-req".to_owned(),
                account_name: Some("shared-name".to_owned()),
                protocol: "openai".to_owned(),
                method: "POST".to_owned(),
                path: "/v1/chat/completions".to_owned(),
                query: None,
                model: Some("alice-model".to_owned()),
                status: "completed".to_owned(),
                status_code: Some(200),
                latency_ms: Some(20),
                error_message: None,
                request_body: None,
                response_body: None,
                request_body_size: None,
                response_body_size: None,
                upstream_protocol: Some("openai".to_owned()),
                upstream_path: Some("/chat/completions".to_owned()),
                client_api: Some("openai_compatible".to_owned()),
                request_surface: Some("OPENAI_CHAT_COMPLETIONS".to_owned()),
                target_surface: Some("OPENAI_CHAT_COMPLETIONS".to_owned()),
                plugin_policy: Some("auto".to_owned()),
                plugin_id: None,
                model_vendor: Some("openai".to_owned()),
                metadata: None,
                streaming: false,
                attempt_count: 1,
            })
            .await
            .unwrap();
        store
            .insert_usage(&NewUsage {
                user_id: 1,
                request_id: "alice-req".to_owned(),
                model: Some("alice-model".to_owned()),
                prompt_tokens: Some(10),
                completion_tokens: Some(5),
                total_tokens: Some(15),
            })
            .await
            .unwrap();

        assert_eq!(
            store
                .list_invocations_for_user(1, 10, 0)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store
                .list_invocations_for_user(2, 10, 0)
                .await
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            store.usage_totals_for_user(1).await.unwrap().total_tokens,
            15
        );
        assert_eq!(
            store.usage_totals_for_user(2).await.unwrap().total_tokens,
            0
        );

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn model_route_mappings_are_first_class_user_isolated_account_data() {
        let (url, path) = temp_sqlite_url("model-route-mappings");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let mut route_mappings = BTreeMap::new();
        route_mappings.insert("gpt-5.5".to_owned(), "gemini-2.5-pro".to_owned());
        route_mappings.insert("claude-4".to_owned(), "claude-sonnet-4-20250514".to_owned());
        let account_id = store
            .insert_account(&NewAccount {
                user_id: 7,
                name: "mapped-account".to_owned(),
                provider: "google".to_owned(),
                base_url: "https://generativelanguage.googleapis.com".to_owned(),
                upstream_api_key: "sk-test".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["gemini-*".to_owned(), "claude-*".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: BTreeMap::new(),
                model_route_mappings: route_mappings,
                enabled: true,
            })
            .await
            .unwrap();

        let user_route_mappings = store.list_model_route_mappings_for_user(7).await.unwrap();
        assert_eq!(user_route_mappings.len(), 2);
        assert_eq!(user_route_mappings[0].user_id, 7);
        assert_eq!(user_route_mappings[0].account_id, account_id);
        assert_eq!(user_route_mappings[0].account_name, "mapped-account");
        assert!(user_route_mappings.iter().any(|route_mapping| {
            route_mapping.client_model == "gpt-5.5"
                && route_mapping.upstream_model == "gemini-2.5-pro"
        }));
        assert!(store
            .list_model_route_mappings_for_user(8)
            .await
            .unwrap()
            .is_empty());

        let mut replacement = BTreeMap::new();
        replacement.insert("gpt-5.5".to_owned(), "gemini-2.5-flash".to_owned());
        store
            .update_account_for_user(
                7,
                account_id,
                &NewAccount {
                    user_id: 7,
                    name: "mapped-account".to_owned(),
                    provider: "google".to_owned(),
                    base_url: "https://generativelanguage.googleapis.com".to_owned(),
                    upstream_api_key: "sk-test".to_owned(),
                    upstream_auth_scheme: None,
                    models: vec!["gemini-*".to_owned()],
                    priority: 10,
                    timeout_secs: 120,
                    max_retries: 0,
                    retry_delay_ms: 500,
                    anthropic_version: None,
                    default_headers: BTreeMap::new(),
                    model_route_mappings: replacement,
                    enabled: true,
                },
            )
            .await
            .unwrap();

        let account_route_mappings = store
            .list_model_route_mappings_for_account(7, account_id)
            .await
            .unwrap();
        assert_eq!(account_route_mappings.len(), 1);
        assert_eq!(account_route_mappings[0].client_model, "gpt-5.5");
        assert_eq!(account_route_mappings[0].upstream_model, "gemini-2.5-flash");

        store.delete_account_for_user(7, account_id).await.unwrap();
        assert!(store
            .list_model_route_mappings_for_user(7)
            .await
            .unwrap()
            .is_empty());

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn account_update_can_preserve_first_class_model_route_mappings() {
        let (url, path) = temp_sqlite_url("model-route-mapping-preserve");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let account = NewAccount {
            user_id: 11,
            name: "runtime-route_mapping-account".to_owned(),
            provider: "google".to_owned(),
            base_url: "https://generativelanguage.googleapis.com".to_owned(),
            upstream_api_key: "sk-test".to_owned(),
            upstream_auth_scheme: None,
            models: vec!["gemini-*".to_owned()],
            priority: 10,
            timeout_secs: 120,
            max_retries: 0,
            retry_delay_ms: 500,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            model_route_mappings: BTreeMap::new(),
            enabled: true,
        };
        let account_id = store.insert_account(&account).await.unwrap();
        store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 11,
                account_id,
                account_name: "runtime-route_mapping-account".to_owned(),
                client_model: "gpt-5.5".to_owned(),
                upstream_model: "gemini-2.5-pro".to_owned(),
                enabled: true,
                notes: Some("runtime managed".to_owned()),
            })
            .await
            .unwrap();

        let mut updated = account;
        updated.priority = 20;
        store
            .update_account_for_user_without_route_mapping_sync(11, account_id, &updated)
            .await
            .unwrap();

        let route_mappings = store
            .list_model_route_mappings_for_account(11, account_id)
            .await
            .unwrap();
        assert_eq!(route_mappings.len(), 1);
        assert_eq!(route_mappings[0].client_model, "gpt-5.5");
        assert_eq!(route_mappings[0].upstream_model, "gemini-2.5-pro");
        assert_eq!(route_mappings[0].notes.as_deref(), Some("runtime managed"));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn account_rename_preserves_route_mappings_and_refreshes_account_name_snapshot() {
        let (url, path) = temp_sqlite_url("model-route-mapping-rename");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let account = NewAccount {
            user_id: 12,
            name: "old-name".to_owned(),
            provider: "google".to_owned(),
            base_url: "https://generativelanguage.googleapis.com".to_owned(),
            upstream_api_key: "sk-test".to_owned(),
            upstream_auth_scheme: None,
            models: vec!["gemini-*".to_owned()],
            priority: 10,
            timeout_secs: 120,
            max_retries: 0,
            retry_delay_ms: 500,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            model_route_mappings: BTreeMap::new(),
            enabled: true,
        };
        let account_id = store.insert_account(&account).await.unwrap();
        store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 12,
                account_id,
                account_name: "old-name".to_owned(),
                client_model: "gpt-5.5".to_owned(),
                upstream_model: "gemini-2.5-pro".to_owned(),
                enabled: true,
                notes: None,
            })
            .await
            .unwrap();

        let mut renamed = account;
        renamed.name = "new-name".to_owned();
        store
            .update_account_for_user_without_route_mapping_sync(12, account_id, &renamed)
            .await
            .unwrap();

        let route_mappings = store
            .list_model_route_mappings_for_account(12, account_id)
            .await
            .unwrap();
        assert_eq!(route_mappings.len(), 1);
        assert_eq!(route_mappings[0].account_name, "new-name");
        assert_eq!(route_mappings[0].client_model, "gpt-5.5");

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn sqlite_model_route_mapping_upsert_returns_existing_id_on_conflict_update() {
        let (url, path) = temp_sqlite_url("model-route-mapping-upsert-id");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let account_id = store
            .insert_account(&NewAccount {
                user_id: 13,
                name: "mapped-account".to_owned(),
                provider: "google".to_owned(),
                base_url: "https://generativelanguage.googleapis.com".to_owned(),
                upstream_api_key: "sk-test".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["gemini-*".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: BTreeMap::new(),
                model_route_mappings: BTreeMap::new(),
                enabled: true,
            })
            .await
            .unwrap();

        let first_id = store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 13,
                account_id,
                account_name: "mapped-account".to_owned(),
                client_model: "gpt-5.5".to_owned(),
                upstream_model: "gemini-2.5-pro".to_owned(),
                enabled: true,
                notes: None,
            })
            .await
            .unwrap();
        let second_id = store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 13,
                account_id,
                account_name: "mapped-account".to_owned(),
                client_model: "claude-4".to_owned(),
                upstream_model: "gemini-2.5-flash".to_owned(),
                enabled: true,
                notes: None,
            })
            .await
            .unwrap();
        let updated_first_id = store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 13,
                account_id,
                account_name: "mapped-account".to_owned(),
                client_model: "gpt-5.5".to_owned(),
                upstream_model: "gemini-2.5-flash".to_owned(),
                enabled: true,
                notes: Some("updated".to_owned()),
            })
            .await
            .unwrap();

        assert_ne!(first_id, second_id);
        assert_eq!(updated_first_id, first_id);
        let route_mappings = store
            .list_model_route_mappings_for_account(13, account_id)
            .await
            .unwrap();
        assert!(route_mappings.iter().any(|route_mapping| {
            route_mapping.id == first_id
                && route_mapping.client_model == "gpt-5.5"
                && route_mapping.upstream_model == "gemini-2.5-flash"
        }));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn model_route_mapping_foreign_key_enforces_user_scoped_account_ownership() {
        let (url, path) = temp_sqlite_url("model-route-mapping-account-owner");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        let account_id = store
            .insert_account(&NewAccount {
                user_id: 20,
                name: "owner-account".to_owned(),
                provider: "google".to_owned(),
                base_url: "https://generativelanguage.googleapis.com".to_owned(),
                upstream_api_key: "sk-test".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["gemini-*".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: BTreeMap::new(),
                model_route_mappings: BTreeMap::new(),
                enabled: true,
            })
            .await
            .unwrap();

        let error = store
            .upsert_model_route_mapping(&NewModelRouteMapping {
                user_id: 21,
                account_id,
                account_name: "owner-account".to_owned(),
                client_model: "gpt-5.5".to_owned(),
                upstream_model: "gemini-2.5-pro".to_owned(),
                enabled: true,
                notes: None,
            })
            .await
            .expect_err("cross-user account route_mapping must be rejected");

        assert!(error.to_string().contains("FOREIGN KEY"));
        assert!(store
            .list_model_route_mappings_for_user(21)
            .await
            .unwrap()
            .is_empty());

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn default_store_methods_use_user_id_zero() {
        let (url, path) = temp_sqlite_url("default-user-id");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();

        store
            .insert_account(&NewAccount {
                user_id: DEFAULT_USER_ID,
                name: "default-account".to_owned(),
                provider: "openai".to_owned(),
                base_url: "https://default.example/v1".to_owned(),
                upstream_api_key: "sk-test".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["default-model".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: BTreeMap::new(),
                model_route_mappings: BTreeMap::new(),
                enabled: true,
            })
            .await
            .unwrap();
        store
            .insert_account(&NewAccount {
                user_id: 99,
                name: "other-account".to_owned(),
                provider: "openai".to_owned(),
                base_url: "https://other.example/v1".to_owned(),
                upstream_api_key: "sk-test".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["other-model".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: BTreeMap::new(),
                model_route_mappings: BTreeMap::new(),
                enabled: true,
            })
            .await
            .unwrap();

        let accounts = store.list_accounts().await.unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].user_id, DEFAULT_USER_ID);
        assert_eq!(accounts[0].name, "default-account");

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
