use crate::config::{AppConfig, ArcadiaConfig, ArcadiaSessionConfig, UserRecord};
use anyhow::Context;
pub(crate) use arche_firestore::*;
use firestore::FirestoreDb;
use serde::{Deserialize, Serialize};
use url::Url;

const ROOT_COLLECTION: &str = "v2";
const ARCADIA_SERVICE_DOC: &str = "arcadia";
const AUTH_SERVICE_DOC: &str = "auth";
const AUTH_USERS_COLLECTION: &str = "users";
const CONFIG_BOOTSTRAP_MAX_RETRIES: usize = 8;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct StoredArcadiaConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session: Option<StoredSessionConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct StoredSessionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pepper: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    absolute_ttl_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    idle_ttl_seconds: Option<u64>,
}

pub async fn load_config(db: &Db) -> anyhow::Result<AppConfig> {
    let stored = load_optional_typed::<StoredArcadiaConfig>(
        db.inner(),
        ROOT_COLLECTION,
        ARCADIA_SERVICE_DOC,
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("v2/arcadia config document is required"))?;
    AppConfig {
        arcadia: validate_arcadia_config(stored)?,
    }
    .require()
}

fn validate_arcadia_config(stored: StoredArcadiaConfig) -> anyhow::Result<ArcadiaConfig> {
    let session = stored.session.unwrap_or_default();
    let origin = normalized_text(stored.origin.as_ref())
        .ok_or_else(|| anyhow::anyhow!("v2/arcadia.origin is required"))?;
    let pepper = normalized_text(session.pepper.as_ref())
        .ok_or_else(|| anyhow::anyhow!("v2/arcadia.session.pepper is required"))?;
    Ok(ArcadiaConfig {
        origin: Url::parse(origin).context("v2/arcadia.origin must be a valid URL")?,
        session: ArcadiaSessionConfig {
            pepper: pepper.to_string(),
            absolute_ttl_seconds: session.absolute_ttl_seconds.unwrap_or(43_200),
            idle_ttl_seconds: session.idle_ttl_seconds.unwrap_or(1_800),
        },
    })
}

pub async fn bootstrap_config(db: &Db) -> anyhow::Result<()> {
    for attempt in 0..CONFIG_BOOTSTRAP_MAX_RETRIES {
        if load_optional_typed::<StoredArcadiaConfig>(
            db.inner(),
            ROOT_COLLECTION,
            ARCADIA_SERVICE_DOC,
        )
        .await?
        .is_some()
        {
            return Ok(());
        }
        let stored = StoredArcadiaConfig {
            origin: Some("https://arcadia.example.invalid".into()),
            session: Some(StoredSessionConfig {
                pepper: Some(crate::util::random_urlsafe_string(64)),
                absolute_ttl_seconds: Some(43_200),
                idle_ttl_seconds: Some(1_800),
            }),
        };
        match create_typed(db.inner(), ROOT_COLLECTION, ARCADIA_SERVICE_DOC, &stored).await {
            Ok(()) => return Ok(()),
            Err(error) if should_retry_bootstrap_conflict(&error, "v2/arcadia", attempt) => {}
            Err(error) => return Err(error),
        }
    }
    anyhow::bail!("v2/arcadia bootstrap did not converge")
}

pub(crate) fn auth_parent(db: &FirestoreDb) -> anyhow::Result<String> {
    Ok(db
        .parent_path(ROOT_COLLECTION, AUTH_SERVICE_DOC)?
        .to_string())
}

pub(crate) fn arcadia_parent(db: &FirestoreDb) -> anyhow::Result<String> {
    Ok(format!(
        "{}/{}/{}",
        db.get_documents_path(),
        ROOT_COLLECTION,
        ARCADIA_SERVICE_DOC
    ))
}

pub(crate) async fn load_user_by_id(db: &Db, id: &str) -> anyhow::Result<Option<UserRecord>> {
    Ok(get_stored_obj_at_if_exists(
        db.inner(),
        &auth_parent(db.inner())?,
        AUTH_USERS_COLLECTION,
        id,
    )
    .await?)
}

pub(crate) async fn load_user_by_email(db: &Db, email: &str) -> anyhow::Result<Option<UserRecord>> {
    let users = db
        .inner()
        .fluent()
        .select()
        .from(AUTH_USERS_COLLECTION)
        .parent(&auth_parent(db.inner())?)
        .query()
        .await?;
    let matches = users
        .iter()
        .map(deserialize_stored_document::<UserRecord>)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|user| user.email == email)
        .collect::<Vec<_>>();
    anyhow::ensure!(
        matches.len() <= 1,
        "multiple Arcadia users are configured for the same email"
    );
    Ok(matches.into_iter().next())
}

fn normalized_text(value: Option<&String>) -> Option<&str> {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcadia_config_requires_https_origin() {
        let stored = StoredArcadiaConfig {
            origin: Some("http://arcadia.example.invalid".into()),
            session: Some(StoredSessionConfig {
                pepper: Some("x".repeat(64)),
                ..Default::default()
            }),
        };
        assert!(validate_arcadia_config(stored).is_ok());
    }
}
