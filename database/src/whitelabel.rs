use async_trait::async_trait;

use sqlx::{Error, PgPool};
use std::fmt;
use std::sync::Arc;

use crate::Table;

use futures::TryStreamExt;
use model::Snowflake;

/// A whitelabel row. `token` is the legacy plaintext column and is NULL once the
/// row has been migrated to `token_ciphertext`. This crate never decrypts.
#[derive(sqlx::FromRow, Clone, PartialEq, Eq)]
pub struct WhitelabelBot {
    pub user_id: i64,
    pub bot_id: i64,
    pub public_key: String,
    pub token: Option<String>,
    pub token_ciphertext: Option<Vec<u8>>,
    pub token_key_version: Option<i16>,
}

impl fmt::Debug for WhitelabelBot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WhitelabelBot")
            .field("user_id", &self.user_id)
            .field("bot_id", &self.bot_id)
            .field("public_key", &self.public_key)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field(
                "token_ciphertext",
                &self.token_ciphertext.as_ref().map(|c| c.len()),
            )
            .field("token_key_version", &self.token_key_version)
            .finish()
    }
}

const COLUMNS: &str =
    r#""user_id", "bot_id", "public_key", "token", "token_ciphertext", "token_key_version""#;

pub struct Whitelabel {
    db: Arc<PgPool>,
}

#[async_trait]
impl Table for Whitelabel {
    async fn create_schema(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS whitelabel(
	"user_id" int8 UNIQUE NOT NULL,
	"bot_id" int8 UNIQUE NOT NULL,
    "public_key" CHAR(64) NOT NULL,
	"token" VARCHAR(84) UNIQUE,
	"token_ciphertext" BYTEA,
	"token_key_version" INT2,
	PRIMARY KEY("user_id")
);
ALTER TABLE whitelabel ADD COLUMN IF NOT EXISTS "token_ciphertext" BYTEA;
ALTER TABLE whitelabel ADD COLUMN IF NOT EXISTS "token_key_version" INT2;
ALTER TABLE whitelabel ALTER COLUMN "token" DROP NOT NULL;
CREATE INDEX IF NOT EXISTS whitelabel_bot_id ON whitelabel("bot_id");
        "#,
        )
        .execute(&*self.db)
        .await?;

        Ok(())
    }
}

impl Whitelabel {
    pub fn new(db: Arc<PgPool>) -> Whitelabel {
        Whitelabel { db }
    }

    pub async fn get_user_by_id(&self, user_id: Snowflake) -> Result<Option<WhitelabelBot>, Error> {
        let query = format!(r#"SELECT {COLUMNS} FROM whitelabel WHERE "user_id" = $1"#);

        match sqlx::query_as::<_, WhitelabelBot>(&query)
            .bind(user_id.0 as i64)
            .fetch_one(&*self.db)
            .await
        {
            Ok(bot) => Ok(Some(bot)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_bot_by_id(&self, bot_id: Snowflake) -> Result<Option<WhitelabelBot>, Error> {
        let query = format!(r#"SELECT {COLUMNS} FROM whitelabel WHERE "bot_id" = $1"#);

        match sqlx::query_as::<_, WhitelabelBot>(&query)
            .bind(bot_id.0 as i64)
            .fetch_one(&*self.db)
            .await
        {
            Ok(bot) => Ok(Some(bot)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Reads only the public key, so callers that never need credentials (the
    /// HTTP interactions gateway) cannot accidentally load or hold a token.
    pub async fn get_public_key_by_bot_id(
        &self,
        bot_id: Snowflake,
    ) -> Result<Option<String>, Error> {
        let query = r#"SELECT "public_key" FROM whitelabel WHERE "bot_id" = $1"#;

        sqlx::query_scalar::<_, String>(query)
            .bind(bot_id.0 as i64)
            .fetch_optional(&*self.db)
            .await
    }

    pub async fn get_bots_by_sharder(
        &self,
        sharder_count: u16,
        sharder_id: u16,
    ) -> Result<Vec<WhitelabelBot>, Error> {
        let query = format!(r#"SELECT {COLUMNS} FROM whitelabel WHERE "bot_id" % $1 = $2"#);

        let mut rows = sqlx::query_as::<_, WhitelabelBot>(&query)
            .bind(sharder_count as i32)
            .bind(sharder_id as i32)
            .fetch(&*self.db);

        let mut bots = Vec::new();
        while let Some(row) = rows.try_next().await? {
            bots.push(row);
        }

        Ok(bots)
    }

    pub async fn delete_by_user_id(&self, user_id: Snowflake) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel WHERE "user_id" = $1;"#;

        sqlx::query(query)
            .bind(user_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn delete_by_bot_id(&self, bot_id: Snowflake) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel WHERE "bot_id" = $1;"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    /// Deletes the row only if it still holds exactly the credentials the caller
    /// connected with, so a token rotated mid-connection is not deleted by a
    /// stale authentication failure. Returns whether a row was deleted.
    pub async fn delete_if_unchanged(&self, snapshot: &WhitelabelBot) -> Result<bool, Error> {
        let query = r#"
DELETE FROM whitelabel
WHERE "bot_id" = $1
  AND "token" IS NOT DISTINCT FROM $2
  AND "token_ciphertext" IS NOT DISTINCT FROM $3
  AND "token_key_version" IS NOT DISTINCT FROM $4;
"#;

        let res = sqlx::query(query)
            .bind(snapshot.bot_id)
            .bind(snapshot.token.as_deref())
            .bind(snapshot.token_ciphertext.as_deref())
            .bind(snapshot.token_key_version)
            .execute(&*self.db)
            .await?;

        Ok(res.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_token_material() {
        let bot = WhitelabelBot {
            user_id: 1,
            bot_id: 2,
            public_key: "pk".to_owned(),
            token: Some("legacy-secret-token".to_owned()),
            token_ciphertext: Some(vec![0xde, 0xad, 0xbe, 0xef]),
            token_key_version: Some(3),
        };

        let out = format!("{:?} {:#?}", bot, bot);
        assert!(!out.contains("legacy-secret-token"));
        assert!(!out.contains("222"));
        assert!(!out.contains("173"));
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn debug_handles_absent_token() {
        let bot = WhitelabelBot {
            user_id: 1,
            bot_id: 2,
            public_key: "pk".to_owned(),
            token: None,
            token_ciphertext: None,
            token_key_version: None,
        };

        assert!(format!("{:?}", bot).contains("token: None"));
    }
}
