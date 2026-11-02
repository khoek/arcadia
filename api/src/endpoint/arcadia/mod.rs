use arche_web::error::ApiError;
mod authenticator;
mod cards;
pub(crate) mod iap;
mod vault;

use crate::{
    arcadia::{ARCADIA_DATABASES, ArcadiaDatabases, ReorderError, load_arcadia_user},
    config::ArcadiaConfig,
    firestore::Db,
    path,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequestParts, Request, State},
    http::{
        HeaderMap, HeaderValue, Method,
        header::{CONTENT_TYPE, ORIGIN},
        request::Parts,
    },
    middleware::{self, Next},
    response::Response,
    routing::{get, patch, put},
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};
use tower_http::cors::CorsLayer;

const QR_BODY_LIMIT_BYTES: usize = 9 * 1024 * 1024;
const MAX_NOTES_BYTES: usize = 50 * 1024;

#[derive(Clone)]
pub struct ArcadiaState {
    pub(super) db: Db,
    pub(super) databases: ArcadiaDatabases,
    origin_header: HeaderValue,
    iap: Option<iap::ArcadiaIapAuth>,
}

impl ArcadiaState {
    pub fn new(
        db: Db,
        config: ArcadiaConfig,
        iap: Option<iap::ArcadiaIapAuth>,
    ) -> anyhow::Result<Self> {
        let origin_header = HeaderValue::from_str(&config.origin.origin().ascii_serialization())?;
        Ok(Self {
            db,
            databases: ARCADIA_DATABASES,
            origin_header,
            iap,
        })
    }

    async fn authenticate(&self, headers: &HeaderMap) -> Result<ArcadiaUser, ApiError> {
        let assertion = headers
            .get("x-goog-iap-jwt-assertion")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("authentication required".into()))?;
        let iap = self
            .iap
            .as_ref()
            .ok_or_else(|| ApiError::Unauthorized("IAP authentication is not configured".into()))?;
        let identity = iap.verify(assertion).await.map_err(|error| {
            tracing::warn!(error = ?error, "Arcadia IAP assertion rejected");
            ApiError::Unauthorized("authentication required".into())
        })?;
        let Some(user) = load_arcadia_user(&self.db, &identity.email)
            .await
            .map_err(ApiError::Internal)?
        else {
            return Err(ApiError::Unauthorized("authentication required".into()));
        };
        if user.disabled || user.email != identity.email {
            return Err(ApiError::Unauthorized("authentication required".into()));
        }
        Ok(ArcadiaUser {
            id: user.id,
            email: user.email,
            absolute_expires_unix: identity.expires_unix,
        })
    }
}

pub fn router(state: ArcadiaState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(state.origin_header.clone())
        .allow_credentials(true)
        .allow_headers([CONTENT_TYPE])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .max_age(Duration::from_secs(3600));

    Router::new()
        .route(path::ARCADIA_SESSION, get(get_session))
        .route(
            path::ARCADIA_AUTHENTICATORS,
            get(authenticator::get_authenticators)
                .post(authenticator::post_authenticator)
                .layer(DefaultBodyLimit::max(QR_BODY_LIMIT_BYTES)),
        )
        .route(
            path::ARCADIA_AUTHENTICATOR_ORDER,
            put(authenticator::put_authenticator_order),
        )
        .route(
            path::ARCADIA_AUTHENTICATOR,
            patch(authenticator::patch_authenticator).delete(authenticator::delete_authenticator),
        )
        .route(
            path::ARCADIA_AUTHENTICATOR_CODES,
            get(authenticator::get_authenticator_codes),
        )
        .route(
            path::ARCADIA_VAULT_ENTRIES,
            get(vault::get_entries).post(vault::post_entry),
        )
        .route(path::ARCADIA_VAULT_ENTRY_ORDER, put(vault::put_entry_order))
        .route(
            path::ARCADIA_VAULT_ENTRY,
            get(vault::get_entry)
                .patch(vault::patch_entry)
                .delete(vault::delete_entry),
        )
        .route(
            path::ARCADIA_VAULT_ENTRY_PASSWORD,
            get(vault::get_entry_password),
        )
        .route(
            path::ARCADIA_CARD_ENTRIES,
            get(cards::get_entries)
                .post(cards::post_entry)
                .layer(DefaultBodyLimit::max(cards::CARD_BODY_LIMIT_BYTES)),
        )
        .route(path::ARCADIA_CARD_ENTRY_ORDER, put(cards::put_entry_order))
        .route(
            path::ARCADIA_CARD_ENTRY,
            get(cards::get_entry)
                .patch(cards::patch_entry)
                .delete(cards::delete_entry),
        )
        .route(
            path::ARCADIA_CARD_ENTRY_NUMBER,
            get(cards::get_entry_number),
        )
        .route(path::ARCADIA_CARD_ENTRY_CVC, get(cards::get_entry_cvc))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            enforce_origin,
        ))
        .with_state(state)
}

#[derive(Clone, Debug)]
pub(super) struct ArcadiaUser {
    pub(super) id: String,
    email: String,
    absolute_expires_unix: i64,
}

impl FromRequestParts<ArcadiaState> for ArcadiaUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ArcadiaState,
    ) -> Result<Self, Self::Rejection> {
        state.authenticate(&parts.headers).await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArcadiaSessionResponse {
    user: ArcadiaSessionUser,
    absolute_expires_at: i64,
}

#[derive(Debug, Serialize)]
struct ArcadiaSessionUser {
    id: String,
    email: String,
}

async fn get_session(user: ArcadiaUser) -> Json<ArcadiaSessionResponse> {
    Json(ArcadiaSessionResponse {
        user: ArcadiaSessionUser {
            id: user.id,
            email: user.email,
        },
        absolute_expires_at: user.absolute_expires_unix.saturating_mul(1000),
    })
}

async fn enforce_origin(
    State(state): State<ArcadiaState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if matches!(
        *request.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) && request.headers().get(ORIGIN) != Some(&state.origin_header)
    {
        return Err(ApiError::Forbidden("request origin is not allowed".into()));
    }
    Ok(next.run(request).await)
}

pub(super) fn validate_required_text(
    value: String,
    field: &str,
    max_chars: usize,
) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }
    if value.chars().count() > max_chars {
        return Err(ApiError::BadRequest(format!("{field} is too long")));
    }
    Ok(value.to_string())
}

pub(super) fn validate_optional_text(
    value: Option<String>,
    field: &str,
    max_chars: usize,
    trim: bool,
) -> Result<Option<String>, ApiError> {
    let value = value.map(|value| {
        if trim {
            value.trim().to_string()
        } else {
            value
        }
    });
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max_chars {
        return Err(ApiError::BadRequest(format!("{field} is too long")));
    }
    Ok(Some(value))
}

pub(super) fn update_optional_text(
    current: Option<String>,
    patch: NullablePatch<String>,
    field: &str,
    max_chars: usize,
    trim: bool,
) -> Result<Option<String>, ApiError> {
    match patch {
        NullablePatch::Missing => Ok(current),
        NullablePatch::Set(value) => validate_optional_text(value, field, max_chars, trim),
    }
}

pub(super) fn validate_notes(value: Option<String>) -> Result<Option<String>, ApiError> {
    let value = validate_optional_text(value, "notes", 20_000, false)?;
    if value
        .as_ref()
        .is_some_and(|value| value.len() > MAX_NOTES_BYTES)
    {
        return Err(ApiError::BadRequest("notes are too large".into()));
    }
    Ok(value)
}

pub(super) fn put_optional_string(
    columns: &mut BTreeMap<String, Value>,
    name: &str,
    value: Option<String>,
) {
    columns.insert(name.to_owned(), value.map_or(Value::Null, Value::String));
}

pub(super) fn validate_icon_image(value: Option<String>) -> Result<Option<String>, ApiError> {
    const MAX_ICON_BYTES: usize = 128 * 1024;
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let (prefix, encoded) = value
        .split_once(',')
        .ok_or_else(|| ApiError::BadRequest("iconImage must be an image data URL".into()))?;
    if !matches!(
        prefix,
        "data:image/png;base64"
            | "data:image/jpeg;base64"
            | "data:image/webp;base64"
            | "data:image/gif;base64"
    ) {
        return Err(ApiError::BadRequest(
            "iconImage must be a PNG, JPEG, WebP, or GIF data URL".into(),
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| ApiError::BadRequest("iconImage is not valid base64".into()))?;
    if bytes.len() > MAX_ICON_BYTES {
        return Err(ApiError::BadRequest(
            "iconImage must be no larger than 128 KiB".into(),
        ));
    }
    Ok(Some(value))
}

pub(super) fn accent_for(id: &str) -> &'static str {
    const ACCENTS: [&str; 5] = ["#246fca", "#7057d9", "#d16a3a", "#188f72", "#b44e82"];
    let hash = id.bytes().fold(0x811c9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x01000193)
    });
    ACCENTS[hash as usize % ACCENTS.len()]
}

#[derive(Debug, Default)]
pub(super) enum NullablePatch<T> {
    #[default]
    Missing,
    Set(Option<T>),
}

pub(super) fn map_reorder_error(error: ReorderError) -> ApiError {
    match error {
        ReorderError::InvalidIds => ApiError::Conflict(error.to_string()),
        ReorderError::Internal(error) => ApiError::Internal(error),
    }
}

impl<'de, T> Deserialize<'de> for NullablePatch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Self::Set)
    }
}

#[cfg(test)]
mod tests {
    use super::validate_icon_image;

    #[test]
    fn icons_require_small_raster_data_urls() {
        assert!(
            validate_icon_image(Some("data:image/png;base64,aGVsbG8=".into()))
                .unwrap()
                .is_some()
        );
        validate_icon_image(Some("data:image/svg+xml;base64,aGVsbG8=".into()))
            .expect_err("SVG must not be accepted");
        validate_icon_image(Some("https://example.test/icon.png".into()))
            .expect_err("remote URL must not be accepted");
    }
}
