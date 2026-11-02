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

pub(super) const CARD_BODY_LIMIT_BYTES: usize = 512 * 1024;

const CARD_ENTRY_ID_LENGTH: usize = 24;
const MAX_CARDHOLDER_CHARS: usize = 160;

#[derive(Clone, Debug)]
struct CardRecord {
    id: String,
    name: String,
    number: Option<String>,
    expiry: Option<String>,
    cvc: Option<String>,
    cardholder: Option<String>,
    notes: Option<String>,
    icon_image: Option<String>,
    accent: String,
    position: i64,
    created_unix: i64,
    updated_unix: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEntrySummary {
    id: String,
    name: String,
    last_four: String,
    has_number: bool,
    has_cvc: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiry: Option<String>,
    accent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_image: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEntryDetail {
    id: String,
    name: String,
    last_four: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cvc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cardholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    accent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_image: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct CardNumberResponse {
    number: String,
}

#[derive(Debug, Serialize)]
pub(super) struct CardCvcResponse {
    cvc: String,
}

pub async fn get_entries(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
) -> Result<Json<Vec<CardEntrySummary>>, ApiError> {
    let records = state
        .databases
        .cards
        .list(&state.db, &user.id)
        .await
        .map_err(ApiError::Internal)?
        .into_iter()
        .map(CardRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;
    let entries = records.iter().map(CardEntrySummary::from).collect();
    Ok(Json(entries))
}

pub async fn get_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<Json<CardEntryDetail>, ApiError> {
    Ok(Json(require_entry(&state, &user.id, &id).await?.into()))
}

pub async fn get_entry_number(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<Json<CardNumberResponse>, ApiError> {
    let number = require_entry(&state, &user.id, &id)
        .await?
        .number
        .ok_or_else(|| ApiError::NotFound("card entry has no number".into()))?;
    Ok(Json(CardNumberResponse { number }))
}

pub async fn get_entry_cvc(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<Json<CardCvcResponse>, ApiError> {
    let cvc = require_entry(&state, &user.id, &id)
        .await?
        .cvc
        .ok_or_else(|| ApiError::NotFound("card entry has no CVC".into()))?;
    Ok(Json(CardCvcResponse { cvc }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCardEntryRequest {
    name: String,
    number: String,
    expiry: String,
    cvc: String,
    #[serde(default)]
    cardholder: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    icon_image: Option<String>,
}

pub async fn post_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Json(request): Json<CreateCardEntryRequest>,
) -> Result<(StatusCode, Json<CardEntrySummary>), ApiError> {
    let name = validate_required_text(request.name, "name", 80)?;
    let number = normalize_card_number(request.number)?;
    let expiry = normalize_expiry(request.expiry)?;
    let cvc = normalize_cvc(request.cvc)?;
    let cardholder =
        validate_optional_text(request.cardholder, "cardholder", MAX_CARDHOLDER_CHARS, true)?;
    let notes = validate_notes(request.notes)?;
    let id = random_urlsafe_string(CARD_ENTRY_ID_LENGTH);
    let now_unix = OffsetDateTime::now_utc().unix_timestamp();
    let record = state
        .databases
        .cards
        .create(
            &state.db,
            &user.id,
            CardRecord {
                id: id.clone(),
                name,
                number: Some(number),
                expiry: Some(expiry),
                cvc: Some(cvc),
                cardholder,
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
    let record = CardRecord::try_from(record).map_err(ApiError::Internal)?;
    Ok((StatusCode::CREATED, Json(CardEntrySummary::from(&record))))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateCardEntryRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    number: NullablePatch<String>,
    #[serde(default)]
    expiry: NullablePatch<String>,
    #[serde(default)]
    cvc: NullablePatch<String>,
    #[serde(default)]
    cardholder: NullablePatch<String>,
    #[serde(default)]
    notes: NullablePatch<String>,
    #[serde(default)]
    icon_image: NullablePatch<String>,
}

pub async fn patch_entry(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
    Json(request): Json<UpdateCardEntryRequest>,
) -> Result<Json<CardEntryDetail>, ApiError> {
    if request.name.is_none()
        && matches!(request.number, NullablePatch::Missing)
        && matches!(request.expiry, NullablePatch::Missing)
        && matches!(request.cvc, NullablePatch::Missing)
        && matches!(request.cardholder, NullablePatch::Missing)
        && matches!(request.notes, NullablePatch::Missing)
        && matches!(request.icon_image, NullablePatch::Missing)
    {
        return Err(ApiError::BadRequest("patch contains no changes".into()));
    }

    let mut record = require_entry(&state, &user.id, &id).await?;
    if let Some(name) = request.name {
        record.name = validate_required_text(name, "name", 80)?;
    }
    record.number = update_card_number(record.number, request.number)?;
    record.expiry = update_expiry(record.expiry, request.expiry)?;
    record.cvc = update_cvc(record.cvc, request.cvc)?;
    record.cardholder = update_optional_text(
        record.cardholder,
        request.cardholder,
        "cardholder",
        MAX_CARDHOLDER_CHARS,
        true,
    )?;
    record.notes = match request.notes {
        NullablePatch::Missing => record.notes,
        NullablePatch::Set(value) => validate_notes(value)?,
    };
    if let NullablePatch::Set(icon_image) = request.icon_image {
        record.icon_image = validate_icon_image(icon_image)?;
    }
    record.updated_unix = OffsetDateTime::now_utc().unix_timestamp();
    let stored: DatabaseRecord = record.clone().into();
    if !state
        .databases
        .cards
        .update(&state.db, &user.id, &stored)
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound("card entry not found".into()));
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
        .cards
        .delete(&state.db, &user.id, &id)
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound("card entry not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardEntryOrderRequest {
    ids: Vec<String>,
}

pub async fn put_entry_order(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Json(request): Json<CardEntryOrderRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .databases
        .cards
        .reorder(&state.db, &user.id, &request.ids)
        .await
        .map_err(map_reorder_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn require_entry(
    state: &ArcadiaState,
    user_id: &str,
    id: &str,
) -> Result<CardRecord, ApiError> {
    state
        .databases
        .cards
        .get(&state.db, user_id, id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or_else(|| ApiError::NotFound("card entry not found".into()))
        .and_then(|record| CardRecord::try_from(record).map_err(ApiError::Internal))
}

fn normalize_card_number(value: String) -> Result<String, ApiError> {
    let value: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != '-')
        .collect();
    if !(12..=19).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "card number must contain 12 to 19 digits".into(),
        ));
    }
    Ok(value)
}

fn normalize_expiry(value: String) -> Result<String, ApiError> {
    let compact: String = value
        .trim()
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    let value = if compact.len() == 4 && compact.bytes().all(|byte| byte.is_ascii_digit()) {
        format!("{}/{}", &compact[..2], &compact[2..])
    } else {
        compact
    };
    let Some((month, year)) = value.split_once('/') else {
        return Err(ApiError::BadRequest(
            "expiry must use MM/YY or MM/YYYY format".into(),
        ));
    };
    if month.len() != 2
        || !month.bytes().all(|byte| byte.is_ascii_digit())
        || !year.bytes().all(|byte| byte.is_ascii_digit())
        || !matches!(year.len(), 2 | 4)
    {
        return Err(ApiError::BadRequest(
            "expiry must use MM/YY or MM/YYYY format".into(),
        ));
    }
    let month_number = month
        .parse::<u8>()
        .map_err(|_| ApiError::BadRequest("expiry must use MM/YY or MM/YYYY format".into()))?;
    if !(1..=12).contains(&month_number) {
        return Err(ApiError::BadRequest(
            "expiry must use MM/YY or MM/YYYY format".into(),
        ));
    }
    Ok(value)
}

fn normalize_cvc(value: String) -> Result<String, ApiError> {
    let value = value.trim().to_string();
    if !matches!(value.len(), 3 | 4) || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "CVC must contain 3 or 4 digits".into(),
        ));
    }
    Ok(value)
}

fn update_card_number(
    current: Option<String>,
    patch: NullablePatch<String>,
) -> Result<Option<String>, ApiError> {
    match patch {
        NullablePatch::Missing => Ok(current),
        NullablePatch::Set(Some(value)) => normalize_card_number(value).map(Some),
        NullablePatch::Set(None) => Ok(None),
    }
}

fn update_expiry(
    current: Option<String>,
    patch: NullablePatch<String>,
) -> Result<Option<String>, ApiError> {
    match patch {
        NullablePatch::Missing => Ok(current),
        NullablePatch::Set(Some(value)) => normalize_expiry(value).map(Some),
        NullablePatch::Set(None) => Ok(None),
    }
}

fn update_cvc(
    current: Option<String>,
    patch: NullablePatch<String>,
) -> Result<Option<String>, ApiError> {
    match patch {
        NullablePatch::Missing => Ok(current),
        NullablePatch::Set(Some(value)) => normalize_cvc(value).map(Some),
        NullablePatch::Set(None) => Ok(None),
    }
}

impl From<&CardRecord> for CardEntrySummary {
    fn from(record: &CardRecord) -> Self {
        let last_four = record
            .number
            .as_deref()
            .map(last_four_digits)
            .unwrap_or_default();
        Self {
            id: record.id.clone(),
            name: record.name.clone(),
            last_four,
            has_number: record.number.is_some(),
            has_cvc: record.cvc.is_some(),
            expiry: record.expiry.clone(),
            accent: record.accent.clone(),
            icon_image: record.icon_image.clone(),
        }
    }
}

impl From<CardRecord> for CardEntryDetail {
    fn from(record: CardRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            last_four: record
                .number
                .as_deref()
                .map(last_four_digits)
                .unwrap_or_default(),
            number: record.number,
            expiry: record.expiry,
            cvc: record.cvc,
            cardholder: record.cardholder,
            notes: record.notes,
            accent: record.accent,
            icon_image: record.icon_image,
        }
    }
}

impl TryFrom<DatabaseRecord> for CardRecord {
    type Error = anyhow::Error;

    fn try_from(record: DatabaseRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            id: record.id.clone(),
            name: record.required_string("name")?.to_string(),
            number: record.string("number")?.map(str::to_owned),
            expiry: record.string("expiry")?.map(str::to_owned),
            cvc: record.string("cvc")?.map(str::to_owned),
            cardholder: record.string("cardholder")?.map(str::to_owned),
            notes: record.string("notes")?.map(str::to_owned),
            icon_image: record.string("icon_image")?.map(str::to_owned),
            accent: record.accent,
            position: record.position,
            created_unix: record.created_unix,
            updated_unix: record.updated_unix,
        })
    }
}

impl From<CardRecord> for DatabaseRecord {
    fn from(record: CardRecord) -> Self {
        let mut columns = BTreeMap::new();
        columns.insert("name".into(), Value::String(record.name));
        put_optional_string(&mut columns, "number", record.number);
        put_optional_string(&mut columns, "expiry", record.expiry);
        put_optional_string(&mut columns, "cvc", record.cvc);
        put_optional_string(&mut columns, "cardholder", record.cardholder);
        put_optional_string(&mut columns, "notes", record.notes);
        put_optional_string(&mut columns, "icon_image", record.icon_image);
        let mut stored =
            DatabaseRecord::new(record.id, columns, record.accent, record.created_unix);
        stored.position = record.position;
        stored.updated_unix = record.updated_unix;
        stored
    }
}

fn last_four_digits(value: &str) -> String {
    value
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        CardEntrySummary, CardRecord, DatabaseRecord, last_four_digits, normalize_card_number,
        normalize_cvc, normalize_expiry,
    };

    #[test]
    fn card_number_grouping_is_normalized() {
        assert_eq!(
            normalize_card_number("4111 1111-1111 1111".into()).unwrap(),
            "4111111111111111"
        );
        normalize_card_number("not a card".into()).expect_err("letters must be rejected");
    }

    #[test]
    fn expiry_accepts_compact_and_slash_forms() {
        assert_eq!(normalize_expiry("1230".into()).unwrap(), "12/30");
        assert_eq!(normalize_expiry("12/2030".into()).unwrap(), "12/2030");
        normalize_expiry("13/30".into()).expect_err("invalid month must be rejected");
    }

    #[test]
    fn cvc_requires_three_or_four_digits() {
        assert_eq!(normalize_cvc("123".into()).unwrap(), "123");
        assert_eq!(normalize_cvc("1234".into()).unwrap(), "1234");
        normalize_cvc("12".into()).expect_err("short CVC must be rejected");
    }

    #[test]
    fn summaries_only_expose_card_number_suffix() {
        let summary = CardEntrySummary::from(&CardRecord {
            id: "card".into(),
            name: "Example".into(),
            number: Some("4111111111111111".into()),
            expiry: Some("12/30".into()),
            cvc: Some("123".into()),
            cardholder: None,
            notes: None,
            icon_image: None,
            accent: "#246fca".into(),
            position: 0,
            created_unix: 1,
            updated_unix: 1,
        });
        let value = serde_json::to_value(summary).unwrap();
        assert_eq!(value["lastFour"], "1111");
        assert_eq!(value["hasNumber"], true);
        assert_eq!(value["hasCvc"], true);
        assert!(value.get("number").is_none());
        assert!(value.get("cvc").is_none());
        assert_eq!(last_four_digits("1234"), "1234");
    }

    #[test]
    fn database_adapter_keeps_card_secrets_raw_for_the_authenticated_client() {
        let record = CardRecord {
            id: "card".into(),
            name: "Example".into(),
            number: Some("4111111111111111".into()),
            expiry: Some("12/30".into()),
            cvc: Some("123".into()),
            cardholder: Some("Person Example".into()),
            notes: None,
            icon_image: None,
            accent: "#246fca".into(),
            position: 1,
            created_unix: 1,
            updated_unix: 2,
        };
        let stored: DatabaseRecord = record.clone().into();
        let encoded = serde_json::to_value(&stored).unwrap();
        assert_eq!(encoded["number"], "4111111111111111");
        assert_eq!(encoded["cvc"], "123");
        let decoded = CardRecord::try_from(stored).unwrap();
        assert_eq!(decoded.number, record.number);
        assert_eq!(decoded.cvc, record.cvc);
        assert_eq!(decoded.position, 1);
    }

    #[test]
    fn database_adapter_serializes_cleared_optional_fields_as_null() {
        let record = CardRecord {
            id: "card".into(),
            name: "Example".into(),
            number: None,
            expiry: None,
            cvc: None,
            cardholder: None,
            notes: None,
            icon_image: None,
            accent: "#246fca".into(),
            position: 0,
            created_unix: 1,
            updated_unix: 1,
        };
        let encoded = serde_json::to_value(DatabaseRecord::from(record)).unwrap();
        assert!(encoded["number"].is_null());
        assert!(encoded["cvc"].is_null());
    }
}
