//! Engram — memoria compartida entre agentes, aislada del estado principal.
//!
//! Vive en su propio fichero SQLite (`engram.db`), separado de
//! `db.v2.sqlite`. Esto es deliberado: un fallo, bloqueo o corrupción de
//! Engram nunca debe impedir que una ejecución guarde su propio estado — ver
//! ADR-008 en `docs/agentos/03-DECISIONES.md` y el criterio de salida de la
//! Fase 5 en `docs/agentos/04-ROADMAP.md`. Todas las operaciones devuelven
//! `Result`; ninguna función de esta librería entra en pánico por un fallo
//! de E/S o de base de datos.

pub mod mcp;

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    Pool, Sqlite,
    migrate::MigrateError,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use uuid::Uuid;
use ts_rs::TS;

#[derive(Debug, thiserror::Error)]
pub enum EngramError {
    #[error("engram store unavailable: {0}")]
    Unavailable(#[from] sqlx::Error),
    #[error("engram migration failed: {0}")]
    Migration(#[from] MigrateError),
    #[error("namespace must not be empty")]
    EmptyNamespace,
    #[error("origin must not be empty")]
    EmptyOrigin,
    #[error("content must not be empty")]
    EmptyContent,
    #[error("recipient must not be empty")]
    EmptyRecipient,
    #[error("session must not be empty")]
    EmptySession,
    #[error("project must not be empty")]
    EmptyProject,
    #[error("scope must not be empty")]
    EmptyScope,
    #[error("purpose must not be empty")]
    EmptyPurpose,
    #[error("granted_by must not be empty")]
    EmptyGrantedBy,
    #[error("context grant TTL must be positive")]
    InvalidGrantTtl,
    #[error("context grant not found")]
    GrantNotFound,
    #[error("context grant expired or revoked")]
    GrantExpired,
    #[error("context grant denied: {0}")]
    GrantDenied(String),
}

/// Una entrada de memoria: proyecto (namespace), origen y caducidad opcional,
/// tal como exige el contrato en `docs/agentos/02-ARQUITECTURA.md` (sección
/// "MemoryEntry").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub namespace: String,
    pub origin: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Caducable authorization for one recipient, session, project and purpose.
/// The grant is persisted so every read/write can be checked independently of
/// the caller's in-memory claims.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ContextGrant {
    pub id: Uuid,
    pub recipient_id: String,
    pub session_id: String,
    pub project_id: String,
    pub scope: String,
    pub purpose: String,
    pub expires_at: DateTime<Utc>,
    pub granted_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct MemoryEntryRow {
    id: String,
    namespace: String,
    origin: String,
    content: String,
    created_at: String,
    updated_at: String,
    expires_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ContextGrantRow {
    id: String,
    recipient_id: String,
    session_id: String,
    project_id: String,
    scope: String,
    purpose: String,
    expires_at: String,
    granted_by: String,
    created_at: String,
    revoked_at: Option<String>,
}

impl TryFrom<MemoryEntryRow> for MemoryEntry {
    type Error = EngramError;

    fn try_from(row: MemoryEntryRow) -> Result<Self, Self::Error> {
        let parse = |s: &str| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| sqlx::Error::Decode("invalid timestamp in memory_entries".into()))
        };
        Ok(MemoryEntry {
            id: Uuid::parse_str(&row.id)
                .map_err(|_| sqlx::Error::Decode("invalid id in memory_entries".into()))?,
            namespace: row.namespace,
            origin: row.origin,
            content: row.content,
            created_at: parse(&row.created_at)?,
            updated_at: parse(&row.updated_at)?,
            expires_at: row.expires_at.as_deref().map(parse).transpose()?,
        })
    }
}

impl TryFrom<ContextGrantRow> for ContextGrant {
    type Error = EngramError;

    fn try_from(row: ContextGrantRow) -> Result<Self, Self::Error> {
        let parse = |value: &str| {
            DateTime::parse_from_rfc3339(value)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| sqlx::Error::Decode("invalid timestamp in context_grants".into()))
        };

        if row.revoked_at.is_some() {
            return Err(EngramError::GrantExpired);
        }

        Ok(ContextGrant {
            id: Uuid::parse_str(&row.id)
                .map_err(|_| sqlx::Error::Decode("invalid id in context_grants".into()))?,
            recipient_id: row.recipient_id,
            session_id: row.session_id,
            project_id: row.project_id,
            scope: row.scope,
            purpose: row.purpose,
            expires_at: parse(&row.expires_at)?,
            granted_by: row.granted_by,
            created_at: parse(&row.created_at)?,
        })
    }
}

/// Almacén de memoria Engram. Cada instancia posee su propio pool de
/// conexiones a un fichero SQLite dedicado, en modo WAL para soportar
/// lectores concurrentes mientras un escritor confirma cambios.
#[derive(Clone)]
pub struct MemoryStore {
    pool: Pool<Sqlite>,
}

impl MemoryStore {
    /// Abre (o crea) el almacén en `path`. Falla con `Err` si el fichero no
    /// se puede abrir o las migraciones no se pueden aplicar — nunca entra
    /// en pánico, para que el llamador decida cómo degradar.
    pub async fn open(path: &Path) -> Result<Self, EngramError> {
        let database_url = format!("sqlite://{}", path.to_string_lossy());
        let options: SqliteConnectOptions = database_url
            .parse::<SqliteConnectOptions>()?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// Issues a short-lived grant. The control plane is expected to call this
    /// after human or policy authorization; the MCP server never self-grants.
    #[allow(clippy::too_many_arguments)]
    pub async fn grant_context(
        &self,
        recipient_id: &str,
        session_id: &str,
        project_id: &str,
        scope: &str,
        purpose: &str,
        ttl_seconds: i64,
        granted_by: &str,
    ) -> Result<ContextGrant, EngramError> {
        let recipient_id = recipient_id.trim();
        let session_id = session_id.trim();
        let project_id = project_id.trim();
        let scope = scope.trim();
        let purpose = purpose.trim();
        let granted_by = granted_by.trim();
        if recipient_id.is_empty() {
            return Err(EngramError::EmptyRecipient);
        }
        if session_id.is_empty() {
            return Err(EngramError::EmptySession);
        }
        if project_id.is_empty() {
            return Err(EngramError::EmptyProject);
        }
        if scope.is_empty() {
            return Err(EngramError::EmptyScope);
        }
        if purpose.is_empty() {
            return Err(EngramError::EmptyPurpose);
        }
        if granted_by.is_empty() {
            return Err(EngramError::EmptyGrantedBy);
        }
        if ttl_seconds <= 0 {
            return Err(EngramError::InvalidGrantTtl);
        }
        if !matches!(scope, "memory:read" | "memory:write" | "memory:readwrite") {
            return Err(EngramError::GrantDenied(format!(
                "unsupported scope `{scope}`"
            )));
        }

        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let expires_at = created_at + chrono::Duration::seconds(ttl_seconds);
        sqlx::query(
            "INSERT INTO context_grants
             (id, recipient_id, session_id, project_id, scope, purpose, expires_at, granted_by, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(recipient_id)
        .bind(session_id)
        .bind(project_id)
        .bind(scope)
        .bind(purpose)
        .bind(expires_at.to_rfc3339())
        .bind(granted_by)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(ContextGrant {
            id,
            recipient_id: recipient_id.to_string(),
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            scope: scope.to_string(),
            purpose: purpose.to_string(),
            expires_at,
            granted_by: granted_by.to_string(),
            created_at,
        })
    }

    /// Revokes an active grant immediately. Subsequent attempts to read or
    /// write with this grant will fail with `EngramError::GrantExpired`.
    /// Returns true if the grant was found and revoked, or false if already revoked or not found.
    pub async fn revoke_grant(
        &self,
        grant_id: Uuid,
        revoked_by: &str,
    ) -> Result<bool, EngramError> {
        let revoked_by = revoked_by.trim();
        if revoked_by.is_empty() {
            return Err(EngramError::EmptyGrantedBy);
        }
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE context_grants SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(&now)
        .bind(grant_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Lists currently active (non-expired and not revoked) grants, optionally filtered
    /// by project_id or session_id.
    pub async fn list_active_grants(
        &self,
        project_id: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<Vec<ContextGrant>, EngramError> {
        let now = Utc::now().to_rfc3339();
        let mut query = String::from(
            "SELECT id, recipient_id, session_id, project_id, scope, purpose,
                    expires_at, granted_by, created_at, revoked_at
             FROM context_grants
             WHERE expires_at > ? AND revoked_at IS NULL",
        );
        if project_id.is_some() {
            query.push_str(" AND project_id = ?");
        }
        if session_id.is_some() {
            query.push_str(" AND session_id = ?");
        }
        query.push_str(" ORDER BY created_at DESC");

        let mut q = sqlx::query_as::<_, ContextGrantRow>(&query).bind(&now);
        if let Some(pid) = project_id {
            q = q.bind(pid);
        }
        if let Some(sid) = session_id {
            q = q.bind(sid);
        }

        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(ContextGrant::try_from).collect()
    }

    async fn authorize_grant(
        &self,
        grant_id: Uuid,
        recipient_id: &str,
        session_id: &str,
        project_id: &str,
        required_scope: &str,
    ) -> Result<(), EngramError> {
        let row = sqlx::query_as::<_, ContextGrantRow>(
            "SELECT id, recipient_id, session_id, project_id, scope, purpose,
                    expires_at, granted_by, created_at, revoked_at
             FROM context_grants WHERE id = ?",
        )
        .bind(grant_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(EngramError::GrantNotFound)?;

        let grant = ContextGrant::try_from(row)?;
        if grant.expires_at <= Utc::now() {
            return Err(EngramError::GrantExpired);
        }
        if grant.recipient_id != recipient_id.trim() {
            return Err(EngramError::GrantDenied("recipient mismatch".to_string()));
        }
        if grant.session_id != session_id.trim() {
            return Err(EngramError::GrantDenied("session mismatch".to_string()));
        }
        if grant.project_id != project_id.trim() {
            return Err(EngramError::GrantDenied("project mismatch".to_string()));
        }
        if grant.scope != required_scope && grant.scope != "memory:readwrite" {
            return Err(EngramError::GrantDenied(format!(
                "scope `{}` does not allow `{required_scope}`",
                grant.scope
            )));
        }
        Ok(())
    }

    /// Writes only after the persisted grant matches the caller and project,
    /// and verifies that `origin` corresponds to the authorized `recipient_id`.
    #[allow(clippy::too_many_arguments)]
    pub async fn write_with_grant(
        &self,
        grant_id: Uuid,
        recipient_id: &str,
        session_id: &str,
        namespace: &str,
        origin: &str,
        content: &str,
        ttl_seconds: Option<i64>,
    ) -> Result<MemoryEntry, EngramError> {
        self.authorize_grant(
            grant_id,
            recipient_id,
            session_id,
            namespace,
            "memory:write",
        )
        .await?;

        let origin_trimmed = origin.trim();
        let recipient_trimmed = recipient_id.trim();
        if !origin_trimmed.starts_with(recipient_trimmed) {
            return Err(EngramError::GrantDenied(format!(
                "origin `{origin_trimmed}` does not match recipient `{recipient_trimmed}`"
            )));
        }

        self.write(namespace, origin, content, ttl_seconds).await
    }

    /// Reads only after the persisted grant matches the caller and project.
    pub async fn read_with_grant(
        &self,
        grant_id: Uuid,
        recipient_id: &str,
        session_id: &str,
        namespace: &str,
        limit: i64,
    ) -> Result<Vec<MemoryEntry>, EngramError> {
        self.authorize_grant(grant_id, recipient_id, session_id, namespace, "memory:read")
            .await?;
        self.read(namespace, limit).await
    }

    /// Escribe una entrada nueva. `ttl_seconds` es opcional: `None` significa
    /// que la entrada no caduca por sí sola (sigue siendo borrable vía
    /// `forget`).
    pub async fn write(
        &self,
        namespace: &str,
        origin: &str,
        content: &str,
        ttl_seconds: Option<i64>,
    ) -> Result<MemoryEntry, EngramError> {
        let namespace = namespace.trim();
        let origin = origin.trim();
        let content = content.trim();
        if namespace.is_empty() {
            return Err(EngramError::EmptyNamespace);
        }
        if origin.is_empty() {
            return Err(EngramError::EmptyOrigin);
        }
        if content.is_empty() {
            return Err(EngramError::EmptyContent);
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = ttl_seconds.map(|secs| now + chrono::Duration::seconds(secs));

        sqlx::query(
            "INSERT INTO memory_entries (id, namespace, origin, content, created_at, updated_at, expires_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(namespace)
        .bind(origin)
        .bind(content)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(expires_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(MemoryEntry {
            id,
            namespace: namespace.to_string(),
            origin: origin.to_string(),
            content: content.to_string(),
            created_at: now,
            updated_at: now,
            expires_at,
        })
    }

    /// Lee las entradas vigentes (no caducadas) de un namespace, más
    /// recientes primero. Otro namespace nunca aparece en el resultado —
    /// es el criterio de aislamiento del gate G5.
    pub async fn read(
        &self,
        namespace: &str,
        limit: i64,
    ) -> Result<Vec<MemoryEntry>, EngramError> {
        let namespace = namespace.trim();
        if namespace.is_empty() {
            return Err(EngramError::EmptyNamespace);
        }
        let now = Utc::now().to_rfc3339();

        let rows: Vec<MemoryEntryRow> = sqlx::query_as(
            "SELECT id, namespace, origin, content, created_at, updated_at, expires_at \
             FROM memory_entries \
             WHERE namespace = ? AND (expires_at IS NULL OR expires_at > ?) \
             ORDER BY created_at DESC \
             LIMIT ?",
        )
        .bind(namespace)
        .bind(&now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(MemoryEntry::try_from).collect()
    }

    /// Removes an entry only after authorizing the caller against the entry's
    /// declared project namespace. The namespace is part of the request so a
    /// caller cannot use an id to probe another project's memory.
    pub async fn forget_with_grant(
        &self,
        grant_id: Uuid,
        recipient_id: &str,
        session_id: &str,
        namespace: &str,
        id: Uuid,
    ) -> Result<bool, EngramError> {
        self.authorize_grant(
            grant_id,
            recipient_id,
            session_id,
            namespace,
            "memory:write",
        )
        .await?;

        let result = sqlx::query("DELETE FROM memory_entries WHERE id = ? AND namespace = ?")
            .bind(id.to_string())
            .bind(namespace.trim())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Purga entradas caducadas. No es parte del camino crítico de
    /// lectura/escritura — se puede llamar periódicamente o nunca sin que
    /// `read` deje de ser correcto (ya filtra por `expires_at`).
    pub async fn purge_expired(&self) -> Result<u64, EngramError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "DELETE FROM memory_entries WHERE expires_at IS NOT NULL AND expires_at <= ?",
        )
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

/// Helper reutilizado por el binario `engram-mcp` y por los tests: abre el
/// store en la ubicación por defecto (`asset_dir()/engram.db`, o
/// `ENGRAM_DB_PATH` si está definida).
pub fn default_db_path() -> std::path::PathBuf {
    if let Ok(custom) = std::env::var("ENGRAM_DB_PATH") {
        return std::path::PathBuf::from(custom);
    }
    utils::assets::asset_dir().join("engram.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn open_temp_store() -> (MemoryStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("engram-test.db");
        let store = MemoryStore::open(&path).await.expect("open store");
        (store, dir)
    }

    #[tokio::test]
    async fn namespace_isolation() {
        let (store, _dir) = open_temp_store().await;

        store
            .write("proj-a", "agent-1", "secret to proj-a", None)
            .await
            .expect("write proj-a");
        store
            .write("proj-b", "agent-1", "unrelated to proj-b", None)
            .await
            .expect("write proj-b");

        let proj_a_entries = store.read("proj-a", 10).await.expect("read proj-a");
        assert_eq!(proj_a_entries.len(), 1);
        assert_eq!(proj_a_entries[0].content, "secret to proj-a");

        let proj_b_entries = store.read("proj-b", 10).await.expect("read proj-b");
        assert_eq!(proj_b_entries.len(), 1);
        assert_eq!(proj_b_entries[0].content, "unrelated to proj-b");

        // proj-a never sees proj-b's content and vice versa.
        assert!(
            proj_a_entries
                .iter()
                .all(|e| e.content != "unrelated to proj-b")
        );
    }

    #[tokio::test]
    async fn two_agents_read_the_same_namespace() {
        let (store, _dir) = open_temp_store().await;

        store
            .write("shared-proj", "claude-code", "note from claude", None)
            .await
            .expect("write from claude");
        store
            .write("shared-proj", "opencode", "note from opencode", None)
            .await
            .expect("write from opencode");

        let entries = store.read("shared-proj", 10).await.expect("read");
        assert_eq!(entries.len(), 2);
        let origins: std::collections::HashSet<_> =
            entries.iter().map(|e| e.origin.as_str()).collect();
        assert!(origins.contains("claude-code"));
        assert!(origins.contains("opencode"));
    }

    #[tokio::test]
    async fn concurrent_writes_from_two_connections_are_both_durable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("engram-concurrent.db");

        let store_a = MemoryStore::open(&path).await.expect("open store a");
        let store_b = MemoryStore::open(&path).await.expect("open store b");

        let write_a = store_a.write("shared-proj", "agent-a", "from a", None);
        let write_b = store_b.write("shared-proj", "agent-b", "from b", None);
        let (res_a, res_b) = tokio::join!(write_a, write_b);
        res_a.expect("write a should succeed under WAL concurrency");
        res_b.expect("write b should succeed under WAL concurrency");

        // A third handle reading sees both writers' entries.
        let entries = store_a.read("shared-proj", 10).await.expect("read");
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn expired_entries_are_not_returned() {
        let (store, _dir) = open_temp_store().await;

        store
            .write("proj-a", "agent-1", "already expired", Some(-1))
            .await
            .expect("write expired entry");
        store
            .write("proj-a", "agent-1", "still valid", None)
            .await
            .expect("write valid entry");

        let entries = store.read("proj-a", 10).await.expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "still valid");
    }

    #[tokio::test]
    async fn forget_removes_an_entry_for_correction() {
        let (store, _dir) = open_temp_store().await;

        let grant = store
            .grant_context(
                "agent-1",
                "session-1",
                "proj-a",
                "memory:readwrite",
                "correction",
                60,
                "human",
            )
            .await
            .expect("grant context");

        let entry = store
            .write_with_grant(
                grant.id,
                "agent-1",
                "session-1",
                "proj-a",
                "agent-1",
                "wrong fact",
                None,
            )
            .await
            .expect("write");

        let existed = store
            .forget_with_grant(grant.id, "agent-1", "session-1", "proj-a", entry.id)
            .await
            .expect("forget");
        assert!(existed);

        let entries = store
            .read_with_grant(grant.id, "agent-1", "session-1", "proj-a", 10)
            .await
            .expect("read");
        assert!(entries.is_empty());

        // Forgetting an id that no longer exists is not an error, just false.
        let existed_again = store
            .forget_with_grant(grant.id, "agent-1", "session-1", "proj-a", entry.id)
            .await
            .expect("forget again");
        assert!(!existed_again);
    }

    #[tokio::test]
    async fn engram_down_reports_a_clean_error_not_a_panic() {
        // A path whose parent directory does not exist cannot be opened —
        // this is the "Engram caído" case: it must surface as Err, never
        // panic, so the caller's own execution stays recoverable.
        let bogus_path = std::path::PathBuf::from("Z:\\this\\path\\does\\not\\exist\\engram.db");
        let result = MemoryStore::open(&bogus_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn empty_namespace_or_origin_or_content_is_rejected() {
        let (store, _dir) = open_temp_store().await;

        assert!(store.write("", "agent-1", "x", None).await.is_err());
        assert!(store.write("proj-a", "", "x", None).await.is_err());
        assert!(store.write("proj-a", "agent-1", "", None).await.is_err());
        assert!(store.read("", 10).await.is_err());
    }

    #[tokio::test]
    async fn memory_access_requires_a_matching_active_grant() {
        let (store, _dir) = open_temp_store().await;

        let grant = store
            .grant_context(
                "agent-1",
                "session-1",
                "proj-a",
                "memory:readwrite",
                "handoff",
                60,
                "human",
            )
            .await
            .expect("grant context");

        let entry = store
            .write_with_grant(
                grant.id,
                "agent-1",
                "session-1",
                "proj-a",
                "agent-1",
                "approved context",
                None,
            )
            .await
            .expect("write with grant");

        let entries = store
            .read_with_grant(grant.id, "agent-1", "session-1", "proj-a", 10)
            .await
            .expect("read with grant");
        assert_eq!(entries[0], entry);

        assert!(
            store
                .read_with_grant(grant.id, "other-agent", "session-1", "proj-a", 10)
                .await
                .is_err()
        );
        assert!(
            store
                .read_with_grant(grant.id, "agent-1", "other-session", "proj-a", 10)
                .await
                .is_err()
        );
        assert!(
            store
                .read_with_grant(grant.id, "agent-1", "session-1", "other-project", 10)
                .await
                .is_err()
        );
        assert!(
            store
                .write_with_grant(
                    Uuid::new_v4(),
                    "agent-1",
                    "session-1",
                    "proj-a",
                    "agent-1",
                    "unauthorized",
                    None,
                )
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn expired_grants_are_rejected_even_when_the_entry_is_valid() {
        let (store, _dir) = open_temp_store().await;

        let grant = store
            .grant_context(
                "agent-1",
                "session-1",
                "proj-a",
                "memory:read",
                "inspection",
                1,
                "human",
            )
            .await
            .expect("grant context");

        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        assert!(
            store
                .read_with_grant(grant.id, "agent-1", "session-1", "proj-a", 10)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn revoke_grant_immediately_invalidates_access() {
        let (store, _dir) = open_temp_store().await;

        let grant = store
            .grant_context(
                "agent-1",
                "session-1",
                "proj-a",
                "memory:readwrite",
                "testing revocation",
                3600,
                "human",
            )
            .await
            .expect("grant context");

        // First write succeeds
        let entry = store
            .write_with_grant(
                grant.id,
                "agent-1",
                "session-1",
                "proj-a",
                "agent-1",
                "valid fact",
                None,
            )
            .await
            .expect("write with active grant");

        assert_eq!(entry.content, "valid fact");

        // Revoke the grant
        let revoked = store
            .revoke_grant(grant.id, "human-operator")
            .await
            .expect("revoke grant");
        assert!(revoked);

        // Subsequent read or write must fail with GrantExpired
        let read_result = store
            .read_with_grant(grant.id, "agent-1", "session-1", "proj-a", 10)
            .await;
        assert!(matches!(read_result, Err(EngramError::GrantExpired)));

        let write_result = store
            .write_with_grant(
                grant.id,
                "agent-1",
                "session-1",
                "proj-a",
                "agent-1",
                "post-revocation fact",
                None,
            )
            .await;
        assert!(matches!(write_result, Err(EngramError::GrantExpired)));

        // Revoking a second time returns false (already revoked)
        let revoked_again = store
            .revoke_grant(grant.id, "human-operator")
            .await
            .expect("revoke again");
        assert!(!revoked_again);
    }

    #[tokio::test]
    async fn origin_must_match_recipient_in_write_with_grant() {
        let (store, _dir) = open_temp_store().await;

        let grant = store
            .grant_context(
                "agent-1",
                "session-1",
                "proj-a",
                "memory:readwrite",
                "testing origin check",
                3600,
                "human",
            )
            .await
            .expect("grant context");

        // Writing with forged origin must be denied
        let forged_result = store
            .write_with_grant(
                grant.id,
                "agent-1",
                "session-1",
                "proj-a",
                "human_supervisor",
                "forged content",
                None,
            )
            .await;
        assert!(matches!(forged_result, Err(EngramError::GrantDenied(_))));

        // Writing with valid recipient or recipient sub-identity succeeds
        let valid_result = store
            .write_with_grant(
                grant.id,
                "agent-1",
                "session-1",
                "proj-a",
                "agent-1:subagent",
                "legitimate content",
                None,
            )
            .await;
        assert!(valid_result.is_ok());
    }
}
