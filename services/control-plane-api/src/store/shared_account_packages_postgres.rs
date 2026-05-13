use super::{Json, PostgresStore, Result};
use crate::store::{
    PublishSharedAccountPackageVersionDraft, SharedAccountPackageHeadRecord,
    SharedAccountPackageVersionRecord,
};
use sqlx::Row;

impl PostgresStore {
    pub(super) async fn get_shared_account_package_head(
        &self,
        account_id: &str,
    ) -> Result<Option<SharedAccountPackageHeadRecord>> {
        let row = sqlx::query("SELECT payload FROM shared_account_package_heads WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get::<Json<SharedAccountPackageHeadRecord>, _>("payload").0))
    }

    pub(super) async fn publish_shared_account_package_version(
        &self,
        draft: PublishSharedAccountPackageVersionDraft,
    ) -> Result<(SharedAccountPackageHeadRecord, SharedAccountPackageVersionRecord)> {
        let current = self.get_shared_account_package_head(&draft.account_id).await?;
        let next_version = current.as_ref().map_or(1, |head| head.current_version + 1);
        let version = SharedAccountPackageVersionRecord {
            account_id: draft.account_id.clone(),
            version: next_version,
            file_name: draft.file_name.clone(),
            file_hash: draft.file_hash.clone(),
            import_secret: draft.import_secret.clone(),
            serialized: draft.serialized.clone(),
            base_version: draft.base_version,
            created_at: draft.created_at.clone(),
            updated_by_client_id: draft.updated_by_client_id.clone(),
        };
        let head = SharedAccountPackageHeadRecord {
            account_id: draft.account_id,
            current_version: next_version,
            file_name: draft.file_name,
            file_hash: draft.file_hash,
            import_secret: draft.import_secret,
            serialized: draft.serialized,
            updated_at: draft.created_at,
            updated_by_client_id: draft.updated_by_client_id,
        };
        sqlx::query(
            "INSERT INTO shared_account_package_versions
                (account_id, version, file_name, file_hash, import_secret, serialized, base_version, created_at, updated_by_client_id, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&version.account_id)
        .bind(i64::try_from(version.version).unwrap_or(1))
        .bind(&version.file_name)
        .bind(&version.file_hash)
        .bind(&version.import_secret)
        .bind(&version.serialized)
        .bind(i64::try_from(version.base_version).unwrap_or(0))
        .bind(&version.created_at)
        .bind(&version.updated_by_client_id)
        .bind(Json(&version))
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT INTO shared_account_package_heads
                (account_id, current_version, file_name, file_hash, import_secret, serialized, updated_at, updated_by_client_id, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (account_id) DO UPDATE
             SET current_version = EXCLUDED.current_version,
                 file_name = EXCLUDED.file_name,
                 file_hash = EXCLUDED.file_hash,
                 import_secret = EXCLUDED.import_secret,
                 serialized = EXCLUDED.serialized,
                 updated_at = EXCLUDED.updated_at,
                 updated_by_client_id = EXCLUDED.updated_by_client_id,
                 payload = EXCLUDED.payload",
        )
        .bind(&head.account_id)
        .bind(i64::try_from(head.current_version).unwrap_or(1))
        .bind(&head.file_name)
        .bind(&head.file_hash)
        .bind(&head.import_secret)
        .bind(&head.serialized)
        .bind(&head.updated_at)
        .bind(&head.updated_by_client_id)
        .bind(Json(&head))
        .execute(&self.pool)
        .await?;
        Ok((head, version))
    }
}
