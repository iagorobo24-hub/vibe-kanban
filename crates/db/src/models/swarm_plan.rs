use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use uuid::Uuid;

/// Valid lifecycle values for a stored swarm plan.
///
/// Mirrors the wire form of `services::services::swarm::SwarmPlanStatus`. It is
/// duplicated here rather than imported because `db` cannot depend on
/// `services` (the dependency runs the other way), and the value is persisted
/// as text that must satisfy the table's `CHECK` constraint.
pub const VALID_SWARM_PLAN_STATUSES: [&str; 5] =
    ["DRAFT", "APPROVED", "EXECUTING", "COMPLETED", "FAILED"];

#[derive(Debug, Error)]
pub enum SwarmPlanRecordError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(
        "Estado de plan de swarm inválido '{0}'; se esperaba uno de DRAFT|APPROVED|EXECUTING|COMPLETED|FAILED"
    )]
    InvalidStatus(String),
}

/// One persisted swarm plan.
///
/// The plan itself is stored as an opaque JSON document in `plan_json`. This is
/// deliberate: the subtasks are not relational entities yet — nothing queries
/// them individually or joins them against telemetry at the SQL level. Only
/// `goal_id` and `status` are promoted to columns, because those are the fields
/// that get filtered and ordered.
///
/// Note the absence of a `TS` derive: this is an internal persistence record,
/// not part of the HTTP surface, so it has no business in `shared/types.ts`.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SwarmPlanRecord {
    pub goal_id: Uuid,
    pub plan_json: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SwarmPlanRecord {
    fn validate_status(status: &str) -> Result<(), SwarmPlanRecordError> {
        if VALID_SWARM_PLAN_STATUSES.contains(&status) {
            Ok(())
        } else {
            Err(SwarmPlanRecordError::InvalidStatus(status.to_string()))
        }
    }

    /// Insert a plan, or replace the document of an existing one.
    ///
    /// `created_at` is preserved across an upsert: the moment a plan was first
    /// stored does not change because its subtasks progressed. `updated_at` is
    /// refreshed by the database.
    pub async fn upsert(
        pool: &SqlitePool,
        goal_id: Uuid,
        plan_json: &str,
        status: &str,
    ) -> Result<Self, SwarmPlanRecordError> {
        Self::validate_status(status)?;

        sqlx::query(
            r#"
            INSERT INTO swarm_plans (goal_id, plan_json, status, created_at, updated_at)
            VALUES (?, ?, ?, datetime('now', 'subsec'), datetime('now', 'subsec'))
            ON CONFLICT(goal_id) DO UPDATE SET
                plan_json  = excluded.plan_json,
                status     = excluded.status,
                updated_at = datetime('now', 'subsec')
            "#,
        )
        .bind(goal_id)
        .bind(plan_json)
        .bind(status)
        .execute(pool)
        .await?;

        Self::find(pool, goal_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound.into())
    }

    pub async fn find(
        pool: &SqlitePool,
        goal_id: Uuid,
    ) -> Result<Option<Self>, SwarmPlanRecordError> {
        let record = sqlx::query_as::<_, Self>(
            "SELECT goal_id, plan_json, status, created_at, updated_at FROM swarm_plans WHERE goal_id = ?",
        )
        .bind(goal_id)
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }

    /// Update the lifecycle status of an existing plan.
    ///
    /// Returns `false` when no such plan exists. It deliberately does **not**
    /// create one: a status transition for an unknown goal is a caller bug, and
    /// materialising a plan out of nothing would hide it.
    pub async fn update_status(
        pool: &SqlitePool,
        goal_id: Uuid,
        status: &str,
    ) -> Result<bool, SwarmPlanRecordError> {
        Self::validate_status(status)?;

        let result = sqlx::query(
            "UPDATE swarm_plans SET status = ?, updated_at = datetime('now', 'subsec') WHERE goal_id = ?",
        )
        .bind(status)
        .bind(goal_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// All stored plans, oldest first.
    ///
    /// `goal_id` breaks ties so the order is total and therefore testable, even
    /// if two plans land in the same subsecond bucket.
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Self>, SwarmPlanRecordError> {
        let records = sqlx::query_as::<_, Self>(
            "SELECT goal_id, plan_json, status, created_at, updated_at FROM swarm_plans ORDER BY created_at ASC, goal_id ASC",
        )
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    pub async fn count(pool: &SqlitePool) -> Result<i64, SwarmPlanRecordError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM swarm_plans")
            .fetch_one(pool)
            .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::{
        SqlitePool,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    };

    use super::*;

    /// A real SQLite database in a temp dir with the real migrations applied.
    ///
    /// This does not touch `dev_assets/db.v2.sqlite`: that is the reason the
    /// project refuses to instantiate `LocalDeployment` in tests.
    async fn create_test_db() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("swarm-plans-test.db");
        let options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.to_string_lossy()))
                .expect("options")
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .expect("pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        (pool, dir)
    }

    fn payload(marker: &str) -> String {
        format!(r#"{{"goal":"{marker}"}}"#)
    }

    #[tokio::test]
    async fn upsert_then_find_returns_the_same_document_and_status() {
        let (pool, _dir) = create_test_db().await;
        let goal_id = Uuid::new_v4();

        let stored = SwarmPlanRecord::upsert(&pool, goal_id, &payload("a"), "DRAFT")
            .await
            .expect("upsert");

        assert_eq!(stored.goal_id, goal_id);
        assert_eq!(stored.status, "DRAFT");

        let fetched = SwarmPlanRecord::find(&pool, goal_id)
            .await
            .expect("find")
            .expect("el plan debe estar persistido");
        assert_eq!(fetched.plan_json, payload("a"));
        assert_eq!(fetched.status, "DRAFT");
    }

    #[tokio::test]
    async fn find_returns_none_for_an_unknown_goal() {
        let (pool, _dir) = create_test_db().await;

        assert!(
            SwarmPlanRecord::find(&pool, Uuid::new_v4())
                .await
                .expect("find")
                .is_none()
        );
    }

    #[tokio::test]
    async fn upsert_replaces_the_document_but_preserves_created_at() {
        let (pool, _dir) = create_test_db().await;
        let goal_id = Uuid::new_v4();

        let first = SwarmPlanRecord::upsert(&pool, goal_id, &payload("before"), "DRAFT")
            .await
            .expect("primer upsert");
        let second = SwarmPlanRecord::upsert(&pool, goal_id, &payload("after"), "APPROVED")
            .await
            .expect("segundo upsert");

        assert_eq!(second.plan_json, payload("after"));
        assert_eq!(second.status, "APPROVED");
        assert_eq!(
            second.created_at, first.created_at,
            "el momento de creación del plan no cambia porque el plan avance"
        );
        assert_eq!(
            SwarmPlanRecord::count(&pool).await.expect("count"),
            1,
            "un upsert sobre un plan existente no debe duplicar la fila"
        );
    }

    #[tokio::test]
    async fn update_status_only_touches_existing_plans_and_never_creates_one() {
        let (pool, _dir) = create_test_db().await;
        let goal_id = Uuid::new_v4();
        SwarmPlanRecord::upsert(&pool, goal_id, &payload("a"), "DRAFT")
            .await
            .expect("upsert");

        assert!(
            SwarmPlanRecord::update_status(&pool, goal_id, "APPROVED")
                .await
                .expect("update"),
            "actualizar un plan existente debe devolver true"
        );
        assert_eq!(
            SwarmPlanRecord::find(&pool, goal_id)
                .await
                .expect("find")
                .expect("plan")
                .status,
            "APPROVED"
        );

        assert!(
            !SwarmPlanRecord::update_status(&pool, Uuid::new_v4(), "APPROVED")
                .await
                .expect("update"),
            "actualizar un plan inexistente debe devolver false"
        );
        assert_eq!(
            SwarmPlanRecord::count(&pool).await.expect("count"),
            1,
            "un update sobre un goal desconocido no debe crear la fila"
        );
    }

    #[tokio::test]
    async fn list_returns_plans_oldest_first() {
        let (pool, _dir) = create_test_db().await;

        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let third = Uuid::new_v4();

        for (goal_id, marker) in [(first, "1"), (second, "2"), (third, "3")] {
            SwarmPlanRecord::upsert(&pool, goal_id, &payload(marker), "DRAFT")
                .await
                .expect("upsert");
        }

        let listed: Vec<Uuid> = SwarmPlanRecord::list(&pool)
            .await
            .expect("list")
            .iter()
            .map(|record| record.goal_id)
            .collect();

        assert_eq!(listed, vec![first, second, third]);
    }

    #[tokio::test]
    async fn an_invalid_status_is_rejected_before_reaching_the_database() {
        let (pool, _dir) = create_test_db().await;

        let err = SwarmPlanRecord::upsert(&pool, Uuid::new_v4(), &payload("a"), "BOGUS")
            .await
            .expect_err("un estado inválido no debe persistirse");
        assert!(
            matches!(err, SwarmPlanRecordError::InvalidStatus(ref s) if s == "BOGUS"),
            "el error debe nombrar el estado inválido: {err}"
        );

        assert_eq!(
            SwarmPlanRecord::count(&pool).await.expect("count"),
            0,
            "el rechazo debe ocurrir antes de escribir"
        );
    }
}
