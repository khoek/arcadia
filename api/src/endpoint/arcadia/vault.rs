use super::{
    ArcadiaState, ArcadiaUser, NullablePatch, accent_for, map_reorder_error, put_optional_string,
    update_optional_text, validate_icon_image, validate_notes, validate_optional_text,
    validate_required_text,
};
use crate::arcadia::DatabaseRecord;
use crate::util::random_urlsafe_string;
use arche_web::error::ApiError;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use time::OffsetDateTime;

const VAULT_ENTRY_ID_LENGTH: usize = 24;

#[derive(Clone)]
struct VaultRecord {
    id: String,
    name: String,
    username: Option<String>,
    password: Option<String>,
    notes: Option<String>,
    icon_image: Option<String>,
    accent: String,
    position: i64,
    created_unix: i64,
    updated_unix: i64,
}

impl TryFrom<DatabaseRecord> for VaultRecord {
    type Error = anyhow::Error;

    fn try_from(record: DatabaseRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            id: record.id.clone(),
            name: record.required_string("name")?.to_owned(),
            username: record.string("username")?.map(str::to_owned),
            password: record.string("password")?.map(str::to_owned),
            notes: record.string("notes")?.map(str::to_owned),
            icon_image: record.string("icon_image")?.map(str::to_owned),
            accent: record.accent,
            position: record.position,
            created_unix: record.created_unix,
            updated_unix: record.updated_unix,
        })
    }
}

impl From<VaultRecord> for DatabaseRecord {
    fn from(record: VaultRecord) -> Self {
        let mut columns = BTreeMap::new();
        columns.insert("name".into(), Value::String(record.name));
        put_optional_string(&mut columns, "username", record.username);
        put_optional_string(&mut columns, "password", record.password);
        put_optional_string(&mut columns, "notes", record.notes);
        put_optional_string(&mut columns, "icon_image", record.icon_image);
        let mut stored =
            DatabaseRecord::new(record.id, columns, record.accent, record.created_unix);
        stored.position = record.position;
        stored.updated_unix = record.updated_unix;
        stored
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntrySummary {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    has_password: bool,
    accent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_image: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntryDetail {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    accent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_image: Option<String>,
}

pub async fn get_entries(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
) -> Result<Json<Vec<VaultEntrySummary>>, ApiError> {
    let records = state
        .databases
        .passwords
        .list(&state.db, &user.id)
        .await
        .map_err(ApiError::Internal)?;
    let entries = records
        .into_iter()
        .map(VaultRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?
        .into_iter()
        .map(|record| VaultEntrySummary {
            id: record.id,
            name: record.name,
            username: record.username,
            has_password: record.password.is_some(),
            accent: record.accent,
            icon_image: record.icon_image,
        })
        .collect();
    Ok(Json(entries))
}

pub async fn get_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<Json<VaultEntryDetail>, ApiError> {
    let record = require_entry(&state, &user.id, &id).await?;
    Ok(Json(record.into()))
}

#[derive(Debug, Serialize)]
pub(super) struct VaultPasswordResponse {
    password: String,
}

pub async fn get_entry_password(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<Json<VaultPasswordResponse>, ApiError> {
    let record = require_entry(&state, &user.id, &id).await?;
    let password = record
        .password
        .ok_or_else(|| ApiError::NotFound("password entry has no password".into()))?;
    Ok(Json(VaultPasswordResponse { password }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVaultEntryRequest {
    name: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    icon_image: Option<String>,
}

pub async fn post_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Json(request): Json<CreateVaultEntryRequest>,
) -> Result<(StatusCode, Json<VaultEntrySummary>), ApiError> {
    let name = validate_required_text(request.name, "name", 80)?;
    let username = validate_optional_text(request.username, "username", 320, true)?;
    let password = validate_optional_text(request.password, "password", 1024, false)?;
    let notes = validate_notes(request.notes)?;
    let id = random_urlsafe_string(VAULT_ENTRY_ID_LENGTH);
    let now_unix = OffsetDateTime::now_utc().unix_timestamp();
    let record = state
        .databases
        .passwords
        .create(
            &state.db,
            &user.id,
            VaultRecord {
                id: id.clone(),
                name,
                username: username.clone(),
                password: password.clone(),
                notes,
                icon_image: validate_icon_image(request.icon_image)?,
                accent: accent_for(&id).into(),
                position: 0,
                created_unix: now_unix,
                updated_unix: now_unix,
            }
            .into(),
        )
        .await
        .map_err(ApiError::Internal)?;
    let record = VaultRecord::try_from(record).map_err(ApiError::Internal)?;
    Ok((
        StatusCode::CREATED,
        Json(VaultEntrySummary {
            id: record.id,
            name: record.name,
            username,
            has_password: password.is_some(),
            accent: record.accent,
            icon_image: record.icon_image,
        }),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateVaultEntryRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    username: NullablePatch<String>,
    #[serde(default)]
    password: NullablePatch<String>,
    #[serde(default)]
    notes: NullablePatch<String>,
    #[serde(default)]
    icon_image: NullablePatch<String>,
}

pub async fn patch_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateVaultEntryRequest>,
) -> Result<Json<VaultEntryDetail>, ApiError> {
    if request.name.is_none()
        && matches!(request.username, NullablePatch::Missing)
        && matches!(request.password, NullablePatch::Missing)
        && matches!(request.notes, NullablePatch::Missing)
        && matches!(request.icon_image, NullablePatch::Missing)
    {
        return Err(ApiError::BadRequest("patch contains no changes".into()));
    }
    let mut record = require_entry(&state, &user.id, &id).await?;
    if let Some(name) = request.name {
        record.name = validate_required_text(name, "name", 80)?;
    }
    record.username =
        update_optional_text(record.username, request.username, "username", 320, true)?;
    record.password =
        update_optional_text(record.password, request.password, "password", 1024, false)?;
    record.notes = match request.notes {
        NullablePatch::Missing => record.notes,
        NullablePatch::Set(value) => validate_notes(value)?,
    };
    if let NullablePatch::Set(icon_image) = request.icon_image {
        record.icon_image = validate_icon_image(icon_image)?;
    }
    record.updated_unix = OffsetDateTime::now_utc().unix_timestamp();
    if !state
        .databases
        .passwords
        .update(&state.db, &user.id, &record.clone().into())
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound("password entry not found".into()));
    }
    Ok(Json(record.into()))
}

pub async fn delete_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if !state
        .databases
        .passwords
        .delete(&state.db, &user.id, &id)
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound("password entry not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultEntryOrderRequest {
    ids: Vec<String>,
}

pub async fn put_entry_order(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Json(request): Json<VaultEntryOrderRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .databases
        .passwords
        .reorder(&state.db, &user.id, &request.ids)
        .await
        .map_err(map_reorder_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn require_entry(
    state: &ArcadiaState,
    user_id: &str,
    id: &str,
) -> Result<VaultRecord, ApiError> {
    state
        .databases
        .passwords
        .get(&state.db, user_id, id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("password entry not found".into()))
        .and_then(|record| VaultRecord::try_from(record).map_err(ApiError::Internal))
}

impl From<VaultRecord> for VaultEntryDetail {
    fn from(record: VaultRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            username: record.username,
            password: record.password,
            notes: record.notes,
            accent: record.accent,
            icon_image: record.icon_image,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VaultRecord, validate_notes};
    use crate::arcadia::DatabaseRecord;

    #[test]
    fn empty_notes_are_absent() {
        assert_eq!(validate_notes(Some(String::new())).unwrap(), None);
    }

    #[test]
    fn notes_obey_the_firestore_payload_limit() {
        validate_notes(Some("x".repeat(20_000))).unwrap();
        validate_notes(Some("😀".repeat(20_000))).expect_err("large UTF-8 notes must fail");
    }

    #[test]
    fn database_adapter_keeps_password_fields_raw() {
        let record = VaultRecord {
            id: "entry".into(),
            name: "Example".into(),
            username: Some("person@example.com".into()),
            password: Some("correct horse battery staple".into()),
            notes: None,
            icon_image: None,
            accent: "#246fca".into(),
            position: 1,
            created_unix: 1,
            updated_unix: 2,
        };
        let stored: DatabaseRecord = record.clone().into();
        let encoded = serde_json::to_value(&stored).unwrap();
        assert_eq!(encoded["username"], "person@example.com");
        assert_eq!(encoded["password"], "correct horse battery staple");
        let decoded = VaultRecord::try_from(stored).unwrap();
        assert_eq!(decoded.password, record.password);
        assert_eq!(decoded.position, 1);
    }
}
