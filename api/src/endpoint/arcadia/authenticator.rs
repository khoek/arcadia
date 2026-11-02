use super::{
    ArcadiaState, ArcadiaUser, NullablePatch, accent_for, map_reorder_error, put_optional_string,
    validate_icon_image, validate_required_text,
};
use crate::arcadia::{ColumnType, DatabaseRecord};
use crate::util::random_urlsafe_string;
use arche_web::error::ApiError;
use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use image::{ImageReader, Limits};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Cursor};
use time::OffsetDateTime;
use totp_rs::{Algorithm, Builder, Secret, Totp};
use url::Url;

const AUTHENTICATOR_ID_LENGTH: usize = 24;
const QR_FILE_LIMIT_BYTES: usize = 8 * 1024 * 1024;
const TOTP_DIGITS: u8 = 6;
const TOTP_PERIOD_SECONDS: u64 = 30;

#[derive(Clone)]
struct AuthenticatorRecord {
    id: String,
    issuer: String,
    account: String,
    algorithm: Algorithm,
    digits: u8,
    period_seconds: u64,
    secret: Secret,
    icon_image: Option<String>,
    accent: String,
    position: i64,
    created_unix: i64,
    updated_unix: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorResource {
    id: String,
    issuer: String,
    account: String,
    accent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_image: Option<String>,
    period_seconds: u64,
}

impl From<AuthenticatorRecord> for AuthenticatorResource {
    fn from(record: AuthenticatorRecord) -> Self {
        Self {
            id: record.id,
            issuer: record.issuer,
            account: record.account,
            accent: record.accent,
            icon_image: record.icon_image,
            period_seconds: record.period_seconds,
        }
    }
}

impl TryFrom<DatabaseRecord> for AuthenticatorRecord {
    type Error = anyhow::Error;

    fn try_from(record: DatabaseRecord) -> Result<Self, Self::Error> {
        let algorithm = Algorithm::try_from(record.required_string("algorithm")?.to_owned())
            .map_err(|error| anyhow::anyhow!("invalid stored TOTP algorithm: {error}"))?;
        anyhow::ensure!(
            matches!(
                algorithm,
                Algorithm::SHA1 | Algorithm::SHA256 | Algorithm::SHA512
            ),
            "stored TOTP algorithm is not supported"
        );
        let secret = Secret::try_from_base32(record.required_string("secret")?)
            .map_err(|_| anyhow::anyhow!("stored TOTP secret is not valid base32"))?;
        Ok(Self {
            id: record.id.clone(),
            issuer: record.required_string("issuer")?.to_owned(),
            account: record.required_string("account")?.to_owned(),
            algorithm,
            digits: u8::try_from(record.required_u64("digits")?)?,
            period_seconds: record.required_u64("period_seconds")?,
            secret,
            icon_image: record.string("icon_image")?.map(str::to_owned),
            accent: record.accent,
            position: record.position,
            created_unix: record.created_unix,
            updated_unix: record.updated_unix,
        })
    }
}

impl From<AuthenticatorRecord> for DatabaseRecord {
    fn from(record: AuthenticatorRecord) -> Self {
        let mut columns = BTreeMap::new();
        columns.insert("issuer".into(), Value::String(record.issuer));
        columns.insert("account".into(), Value::String(record.account));
        columns.insert(
            "algorithm".into(),
            Value::String(record.algorithm.as_str().to_owned()),
        );
        columns.insert("digits".into(), json!(record.digits));
        columns.insert("period_seconds".into(), json!(record.period_seconds));
        columns.insert("secret".into(), Value::String(record.secret.to_base32()));
        put_optional_string(&mut columns, "icon_image", record.icon_image);
        let mut stored =
            DatabaseRecord::new(record.id, columns, record.accent, record.created_unix);
        stored.position = record.position;
        stored.updated_unix = record.updated_unix;
        stored
    }
}

pub async fn get_authenticators(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
) -> Result<Json<Vec<AuthenticatorResource>>, ApiError> {
    let records = state
        .databases
        .authenticator
        .list(&state.db, &user.id)
        .await
        .map_err(ApiError::Internal)?;
    let records = records
        .into_iter()
        .map(AuthenticatorRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;
    Ok(Json(
        records
            .into_iter()
            .map(AuthenticatorResource::from)
            .collect(),
    ))
}

pub async fn post_authenticator(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    multipart: Multipart,
) -> Result<(StatusCode, Json<AuthenticatorResource>), ApiError> {
    let AuthenticatorMultipart {
        mode,
        issuer,
        account,
        secret,
        qr,
        icon_image,
    } = AuthenticatorMultipart::parse(multipart).await?;
    let (totp, issuer, account) = match mode.as_str() {
        "qr" => {
            if issuer.is_some() || account.is_some() {
                return Err(ApiError::BadRequest(
                    "issuer and account must not be supplied in QR mode".into(),
                ));
            }
            if secret.is_some() {
                return Err(ApiError::BadRequest(
                    "secret must not be supplied in QR mode".into(),
                ));
            }
            let totp = parse_qr_totp(
                qr.ok_or_else(|| ApiError::BadRequest("qr is required in QR mode".into()))?,
            )
            .await?;
            let (issuer, account) = qr_identity(&totp)?;
            (totp, issuer, account)
        }
        "manual" => {
            if qr.is_some() {
                return Err(ApiError::BadRequest(
                    "qr must not be supplied in manual mode".into(),
                ));
            }
            let issuer = validate_required_text(issuer.unwrap_or_default(), "issuer", 80)?;
            let account = validate_required_text(account.unwrap_or_default(), "account", 160)?;
            let totp = parse_manual_totp(
                secret.ok_or_else(|| {
                    ApiError::BadRequest("secret is required in manual mode".into())
                })?,
                &issuer,
                &account,
            )?;
            (totp, issuer, account)
        }
        _ => return Err(ApiError::BadRequest("mode must be qr or manual".into())),
    };
    validate_supported_totp(&totp)?;

    let id = random_urlsafe_string(AUTHENTICATOR_ID_LENGTH);
    let now_unix = OffsetDateTime::now_utc().unix_timestamp();
    let record = state
        .databases
        .authenticator
        .create(
            &state.db,
            &user.id,
            AuthenticatorRecord {
                id: id.clone(),
                issuer,
                account,
                algorithm: totp.algorithm(),
                digits: totp.digits(),
                period_seconds: totp.step(),
                secret: totp.secret().clone(),
                icon_image: validate_icon_image(icon_image)?,
                accent: accent_for(&id).into(),
                position: 0,
                created_unix: now_unix,
                updated_unix: now_unix,
            }
            .into(),
        )
        .await
        .map_err(ApiError::Internal)?;
    let record = AuthenticatorRecord::try_from(record).map_err(ApiError::Internal)?;
    Ok((StatusCode::CREATED, Json(record.into())))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthenticatorPatch {
    #[serde(default)]
    issuer: Option<String>,
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    icon_image: NullablePatch<String>,
}

pub async fn patch_authenticator(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
    Json(patch): Json<AuthenticatorPatch>,
) -> Result<Json<AuthenticatorResource>, ApiError> {
    let Some(mut record) = state
        .databases
        .authenticator
        .get(&state.db, &user.id, &id)
        .await
        .map_err(ApiError::Internal)?
        .map(AuthenticatorRecord::try_from)
        .transpose()
        .map_err(ApiError::Internal)?
    else {
        return Err(ApiError::NotFound("authenticator not found".into()));
    };
    if patch.issuer.is_none()
        && patch.account.is_none()
        && matches!(patch.icon_image, NullablePatch::Missing)
    {
        return Err(ApiError::BadRequest("patch contains no changes".into()));
    }
    if let Some(issuer) = patch.issuer {
        record.issuer = validate_required_text(issuer, "issuer", 80)?;
    }
    if let Some(account) = patch.account {
        record.account = validate_required_text(account, "account", 160)?;
    }
    if let NullablePatch::Set(icon_image) = patch.icon_image {
        record.icon_image = validate_icon_image(icon_image)?;
    }
    record.updated_unix = OffsetDateTime::now_utc().unix_timestamp();
    if !state
        .databases
        .authenticator
        .update(&state.db, &user.id, &record.clone().into())
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound("authenticator not found".into()));
    }
    Ok(Json(record.into()))
}

pub async fn delete_authenticator(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if !state
        .databases
        .authenticator
        .delete(&state.db, &user.id, &id)
        .await
        .map_err(ApiError::Internal)?
    {
        return Err(ApiError::NotFound("authenticator not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatorOrderRequest {
    ids: Vec<String>,
}

pub async fn put_authenticator_order(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
    Json(request): Json<AuthenticatorOrderRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .databases
        .authenticator
        .reorder(&state.db, &user.id, &request.ids)
        .await
        .map_err(map_reorder_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatorCodeBatch {
    server_time: i64,
    codes: Vec<AuthenticatorCodePair>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatorCodePair {
    authenticator_id: String,
    current: AuthenticatorCodePeriod,
    next: AuthenticatorCodePeriod,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthenticatorCodePeriod {
    code: String,
    valid_from: i64,
    valid_until: i64,
}

pub async fn get_authenticator_codes(
    State(state): State<ArcadiaState>,
    user: ArcadiaUser,
) -> Result<Json<AuthenticatorCodeBatch>, ApiError> {
    let now_unix = OffsetDateTime::now_utc().unix_timestamp();
    let now_seconds = u64::try_from(now_unix).map_err(anyhow::Error::from)?;
    // This is the type-specific query for the database's OTP column.  The
    // generic database only returns the stored seed; this handler expands it
    // into the current and next code periods exposed by the REST API.
    state
        .databases
        .authenticator
        .require_column("secret", ColumnType::Otp)
        .map_err(ApiError::Internal)?;
    let records = state
        .databases
        .authenticator
        .list(&state.db, &user.id)
        .await
        .map_err(ApiError::Internal)?;
    let codes = records
        .into_iter()
        .map(AuthenticatorRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?
        .into_iter()
        .map(|record| {
            let id = record.id.clone();
            let totp = totp_from_record(&record)?;
            let current_start = now_seconds / record.period_seconds * record.period_seconds;
            let next_start = current_start
                .checked_add(record.period_seconds)
                .ok_or_else(|| anyhow::anyhow!("TOTP period overflow"))?;
            Ok::<_, anyhow::Error>(AuthenticatorCodePair {
                authenticator_id: id,
                current: code_period(&totp, current_start, next_start)?,
                next: code_period(
                    &totp,
                    next_start,
                    next_start
                        .checked_add(record.period_seconds)
                        .ok_or_else(|| anyhow::anyhow!("TOTP period overflow"))?,
                )?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(ApiError::Internal)?;
    Ok(Json(AuthenticatorCodeBatch {
        server_time: now_unix.saturating_mul(1000),
        codes,
    }))
}

#[derive(Default)]
struct AuthenticatorMultipart {
    mode: String,
    issuer: Option<String>,
    account: Option<String>,
    secret: Option<String>,
    qr: Option<Vec<u8>>,
    icon_image: Option<String>,
}

impl AuthenticatorMultipart {
    async fn parse(mut multipart: Multipart) -> Result<Self, ApiError> {
        let mut input = Self::default();
        let mut seen = std::collections::BTreeSet::new();
        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|error| ApiError::BadRequest(error.body_text()))?
        {
            let name = field
                .name()
                .ok_or_else(|| ApiError::BadRequest("multipart field has no name".into()))?
                .to_string();
            if !seen.insert(name.clone()) {
                return Err(ApiError::BadRequest(format!(
                    "multipart field {name} was supplied more than once"
                )));
            }
            match name.as_str() {
                "mode" => input.mode = multipart_text(field, "mode").await?,
                "issuer" => input.issuer = Some(multipart_text(field, "issuer").await?),
                "account" => input.account = Some(multipart_text(field, "account").await?),
                "secret" => input.secret = Some(multipart_text(field, "secret").await?),
                "iconImage" => input.icon_image = Some(multipart_text(field, "iconImage").await?),
                "qr" => {
                    let bytes = field
                        .bytes()
                        .await
                        .map_err(|error| ApiError::BadRequest(error.body_text()))?;
                    if bytes.is_empty() || bytes.len() > QR_FILE_LIMIT_BYTES {
                        return Err(ApiError::BadRequest(
                            "qr must be a non-empty image no larger than 8 MiB".into(),
                        ));
                    }
                    input.qr = Some(bytes.to_vec());
                }
                _ => {
                    return Err(ApiError::BadRequest(format!(
                        "unknown multipart field {name}"
                    )));
                }
            }
        }
        Ok(input)
    }
}

async fn multipart_text(
    field: axum::extract::multipart::Field<'_>,
    name: &str,
) -> Result<String, ApiError> {
    field
        .text()
        .await
        .map_err(|error| ApiError::BadRequest(format!("invalid {name}: {}", error.body_text())))
}

async fn parse_qr_totp(bytes: Vec<u8>) -> Result<Totp, ApiError> {
    tokio::task::spawn_blocking(move || {
        let mut reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| "QR upload is not a supported image".to_string())?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|_| "QR upload is not a valid PNG, JPEG, WebP, or GIF image".to_string())?
            .to_luma8();
        let mut prepared = rqrr::PreparedImage::prepare(image);
        let grids = prepared.detect_grids();
        if grids.len() != 1 {
            return Err("QR image must contain exactly one QR code".to_string());
        }
        let (_, value) = grids[0]
            .decode()
            .map_err(|_| "QR code could not be decoded".to_string())?;
        parse_qr_totp_value(&value)
    })
    .await
    .map_err(|error| ApiError::Internal(error.into()))?
    .map_err(ApiError::BadRequest)
}

fn parse_qr_totp_value(value: &str) -> Result<Totp, String> {
    // The Google Authenticator Key URI Format permits arbitrary Base32 keys and uses an 80-bit
    // key in its canonical example. The supported runtime properties are checked below.
    // Provider-issued URIs may encode the case-insensitive Base32 secret in lowercase, while
    // totp-rs expects the RFC 4648 uppercase alphabet.
    let mut url =
        Url::parse(value).map_err(|_| "QR code is not a valid TOTP setup code".to_string())?;
    let query = url
        .query_pairs()
        .map(|(key, value)| {
            let key = key.into_owned();
            let value = if key == "secret" {
                value.to_ascii_uppercase()
            } else {
                value.into_owned()
            };
            (key, value)
        })
        .collect::<Vec<_>>();
    if !query.is_empty() {
        url.set_query(None);
        url.query_pairs_mut().extend_pairs(&query);
    }
    Totp::from_url_unchecked(url).map_err(|_| "QR code is not a valid TOTP setup code".to_string())
}

fn parse_manual_totp(secret: String, issuer: &str, account: &str) -> Result<Totp, ApiError> {
    let normalized = secret
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != '-')
        .collect::<String>()
        .trim_end_matches('=')
        .to_ascii_uppercase();
    let secret = Secret::try_from_base32(normalized)
        .map_err(|_| ApiError::BadRequest("secret is not valid base32".into()))?;
    Ok(Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(TOTP_DIGITS)
        .with_skew(0)
        .with_step_duration(TOTP_PERIOD_SECONDS)
        .with_secret(secret)
        .with_issuer(Some(issuer))
        .with_account_name(account)
        .build_noncompliant())
}

fn qr_identity(totp: &Totp) -> Result<(String, String), ApiError> {
    let issuer = totp
        .issuer()
        .ok_or_else(|| ApiError::BadRequest("QR code must include an issuer".into()))?;
    Ok((
        validate_required_text(issuer.to_owned(), "issuer", 80)?,
        validate_required_text(totp.account_name().to_owned(), "account", 160)?,
    ))
}

fn validate_supported_totp(totp: &Totp) -> Result<(), ApiError> {
    if totp.secret().as_bytes().is_empty() {
        return Err(ApiError::BadRequest("TOTP secret must not be empty".into()));
    }
    if totp.digits() != TOTP_DIGITS || totp.step() != TOTP_PERIOD_SECONDS {
        return Err(ApiError::BadRequest(
            "only six-digit TOTP codes with a 30-second period are supported".into(),
        ));
    }
    match totp.algorithm() {
        Algorithm::SHA1 | Algorithm::SHA256 | Algorithm::SHA512 => Ok(()),
        _ => Err(ApiError::BadRequest(
            "TOTP algorithm is not supported".into(),
        )),
    }
}

fn totp_from_record(record: &AuthenticatorRecord) -> anyhow::Result<Totp> {
    anyhow::ensure!(
        !record.secret.as_bytes().is_empty(),
        "stored TOTP secret must not be empty"
    );
    anyhow::ensure!(
        record.digits == TOTP_DIGITS && record.period_seconds == TOTP_PERIOD_SECONDS,
        "stored TOTP configuration is not supported"
    );
    Ok(Builder::new()
        .with_algorithm(record.algorithm)
        .with_digits(record.digits)
        .with_skew(0)
        .with_step_duration(record.period_seconds)
        .with_secret(record.secret.clone())
        // Provider-issued Key URI secrets can be shorter than RFC 4226's
        // generation recommendation; the record has already been validated
        // at import time, so preserve that exact seed when generating codes.
        .build_noncompliant())
}

fn code_period(
    totp: &Totp,
    valid_from_seconds: u64,
    valid_until_seconds: u64,
) -> anyhow::Result<AuthenticatorCodePeriod> {
    Ok(AuthenticatorCodePeriod {
        code: totp.generate(valid_from_seconds).to_string(),
        valid_from: i64::try_from(valid_from_seconds)?.saturating_mul(1000),
        valid_until: i64::try_from(valid_until_seconds)?.saturating_mul(1000),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AuthenticatorRecord, parse_manual_totp, parse_qr_totp_value, qr_identity, totp_from_record,
        validate_supported_totp,
    };
    use crate::arcadia::DatabaseRecord;
    use arche_web::error::ApiError;
    use totp_rs::{Algorithm, Secret};

    #[test]
    fn manual_setup_keys_accept_standard_grouping() {
        let totp = parse_manual_totp(
            "JBSW Y3DP-EHPK 3PXP JBSW-Y3DP EHPK 3PXP".into(),
            "Example",
            "person@example.com",
        )
        .unwrap();
        validate_supported_totp(&totp).unwrap();
        assert_eq!(totp.digits(), 6);
        assert_eq!(totp.step(), 30);
    }

    #[test]
    fn manual_setup_keys_accept_provider_issued_80_bit_secrets() {
        let totp =
            parse_manual_totp("JBSWY3DPEHPK3PXP".into(), "Example", "person@example.com").unwrap();

        validate_supported_totp(&totp).unwrap();
        assert_eq!(totp.secret().as_bytes().len(), 10);
    }

    #[test]
    fn qr_setup_accepts_provider_issued_80_bit_secrets() {
        let totp = parse_qr_totp_value(
            "otpauth://totp/Example:person%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example",
        )
        .unwrap();

        validate_supported_totp(&totp).unwrap();
        assert_eq!(totp.secret().as_bytes().len(), 10);
        assert_eq!(totp.generate(59).to_string(), "996554");
        assert_eq!(
            qr_identity(&totp).unwrap(),
            ("Example".into(), "person@example.com".into())
        );
    }

    #[test]
    fn qr_setup_accepts_lowercase_base32_secrets() {
        let totp = parse_qr_totp_value(
            "otpauth://totp/Example:person%40example.com?secret=jbswy3dpehpk3pxpjbswy3dpehpk3pxp&issuer=Example",
        )
        .unwrap();

        validate_supported_totp(&totp).unwrap();
        assert_eq!(
            totp.secret().to_base32(),
            "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP"
        );
        assert_eq!(totp.generate(59).to_string(), "503347");
    }

    #[test]
    fn imported_setup_keys_must_not_be_empty() {
        let totp = parse_qr_totp_value(
            "otpauth://totp/Example:person%40example.com?secret=&issuer=Example",
        )
        .unwrap();

        assert!(matches!(
            validate_supported_totp(&totp),
            Err(ApiError::BadRequest(message)) if message == "TOTP secret must not be empty"
        ));
    }

    #[test]
    fn stored_provider_issued_secrets_generate_codes() {
        let record = AuthenticatorRecord {
            id: "authenticator".into(),
            issuer: "Example".into(),
            account: "person@example.com".into(),
            algorithm: Algorithm::SHA1,
            digits: 6,
            period_seconds: 30,
            secret: Secret::try_from_base32("JBSWY3DPEHPK3PXP").unwrap(),
            icon_image: None,
            accent: "#246fca".into(),
            position: 0,
            created_unix: 1,
            updated_unix: 1,
        };

        let totp = totp_from_record(&record).unwrap();
        assert_eq!(totp.generate(59).to_string(), "996554");
    }

    #[test]
    fn database_adapter_keeps_otp_seed_as_a_direct_column() {
        let record = AuthenticatorRecord {
            id: "authenticator".into(),
            issuer: "Example".into(),
            account: "person@example.com".into(),
            algorithm: Algorithm::SHA1,
            digits: 6,
            period_seconds: 30,
            secret: Secret::try_from_base32("JBSWY3DPEHPK3PXP").unwrap(),
            icon_image: None,
            accent: "#246fca".into(),
            position: 2,
            created_unix: 1,
            updated_unix: 2,
        };
        let stored: DatabaseRecord = record.clone().into();
        let encoded = serde_json::to_value(&stored).unwrap();
        assert_eq!(encoded["secret"], "JBSWY3DPEHPK3PXP");
        let decoded = AuthenticatorRecord::try_from(stored).unwrap();
        assert_eq!(decoded.secret.to_base32(), record.secret.to_base32());
        assert_eq!(decoded.position, 2);
    }

    #[test]
    fn qr_identity_comes_from_setup_uri() {
        let totp = parse_qr_totp_value(
            "otpauth://totp/Example:person%40example.com?secret=JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP&issuer=Example",
        )
        .unwrap();

        assert_eq!(
            qr_identity(&totp).unwrap(),
            ("Example".into(), "person@example.com".into())
        );
    }

    #[test]
    fn qr_identity_requires_an_issuer() {
        let totp = parse_qr_totp_value(
            "otpauth://totp/person%40example.com?secret=JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP",
        )
        .unwrap();

        assert!(matches!(
            qr_identity(&totp),
            Err(ApiError::BadRequest(message)) if message == "QR code must include an issuer"
        ));
    }
}
