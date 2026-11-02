use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};

const IAP_ISSUER: &str = "https://cloud.google.com/iap";
const IAP_JWKS_URL: &str = "https://www.gstatic.com/iap/verify/public_key-jwk";
const IAP_CLOCK_SKEW: Duration = Duration::from_secs(30);
const IAP_MAX_TOKEN_LIFETIME: Duration = Duration::from_secs(11 * 60);
const IAP_KEY_CACHE_LIFETIME: Duration = Duration::from_secs(60 * 60);
const IAP_UNKNOWN_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const MAX_IAP_ASSERTION_BYTES: usize = 8 * 1024;
const MAX_IAP_JWKS_BYTES: usize = 64 * 1024;
const MAX_IAP_KEYS: usize = 16;

#[derive(Clone)]
pub(crate) struct ArcadiaIapAuth {
    verifier: Arc<IapVerifier>,
}

impl ArcadiaIapAuth {
    pub(crate) async fn from_env() -> anyhow::Result<Option<Self>> {
        let Some(audience) = env::var_os("ARCADIA_IAP_AUDIENCE") else {
            return Ok(None);
        };
        let audience = audience
            .into_string()
            .map_err(|_| anyhow::anyhow!("ARCADIA_IAP_AUDIENCE must be valid UTF-8"))?;
        Ok(Some(Self {
            verifier: Arc::new(IapVerifier::open(audience).await?),
        }))
    }

    pub(super) async fn verify(&self, assertion: &str) -> anyhow::Result<IapIdentity> {
        self.verifier.verify(assertion).await
    }
}

#[derive(Clone, Debug)]
pub(super) struct IapIdentity {
    pub(super) email: String,
    pub(super) expires_unix: i64,
}

struct IapVerifier {
    audience: String,
    client: Client,
    keys: RwLock<CachedKeys>,
    refresh: Mutex<()>,
}

struct CachedKeys {
    by_id: BTreeMap<String, DecodingKey>,
    expires_at: Instant,
    unknown_key_refresh_after: Instant,
}

impl IapVerifier {
    async fn open(audience: String) -> anyhow::Result<Self> {
        anyhow::ensure!(
            valid_backend_service_audience(&audience),
            "ARCADIA_IAP_AUDIENCE is invalid"
        );
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("arcadia-api/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let keys = fetch_keys(&client).await?;
        Ok(Self {
            audience,
            client,
            keys: RwLock::new(keys),
            refresh: Mutex::new(()),
        })
    }

    async fn verify(&self, assertion: &str) -> anyhow::Result<IapIdentity> {
        anyhow::ensure!(
            valid_assertion_envelope(assertion),
            "invalid IAP assertion envelope"
        );
        let header = decode_header(assertion)?;
        anyhow::ensure!(
            header.alg == Algorithm::ES256,
            "IAP assertion uses an unexpected algorithm"
        );
        let kid = header
            .kid
            .ok_or_else(|| anyhow::anyhow!("IAP assertion has no key id"))?;
        anyhow::ensure!(valid_key_id(&kid), "IAP assertion key id is invalid");
        let key = self.decoding_key(&kid).await?;
        let mut validation = Validation::new(Algorithm::ES256);
        validation.leeway = IAP_CLOCK_SKEW.as_secs();
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
        validation.set_issuer(&[IAP_ISSUER]);
        validation.set_audience(&[&self.audience]);
        let claims = decode::<IapClaims>(assertion, &key, &validation)?.claims;
        validate_claims(&claims, &self.audience)
    }

    async fn decoding_key(&self, kid: &str) -> anyhow::Result<DecodingKey> {
        {
            let keys = self.keys.read().await;
            let now = Instant::now();
            if keys.expires_at > now {
                if let Some(key) = keys.by_id.get(kid) {
                    return Ok(key.clone());
                }
                if keys.unknown_key_refresh_after > now {
                    anyhow::bail!("IAP assertion key id is unknown");
                }
            }
        }
        let _refresh = self.refresh.lock().await;
        {
            let keys = self.keys.read().await;
            let now = Instant::now();
            if keys.expires_at > now {
                if let Some(key) = keys.by_id.get(kid) {
                    return Ok(key.clone());
                }
                if keys.unknown_key_refresh_after > now {
                    anyhow::bail!("IAP assertion key id is unknown");
                }
            }
        }
        let refreshed = fetch_keys(&self.client).await?;
        let key = refreshed.by_id.get(kid).cloned();
        *self.keys.write().await = refreshed;
        key.ok_or_else(|| anyhow::anyhow!("IAP assertion key id is unknown"))
    }
}

#[derive(Debug, Deserialize)]
struct IapClaims {
    aud: String,
    email: String,
    exp: u64,
    iat: u64,
    iss: String,
    sub: String,
}

fn validate_claims(claims: &IapClaims, audience: &str) -> anyhow::Result<IapIdentity> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let lifetime = claims
        .exp
        .checked_sub(claims.iat)
        .ok_or_else(|| anyhow::anyhow!("IAP assertion lifetime is invalid"))?;
    anyhow::ensure!(claims.aud == audience, "IAP assertion audience is invalid");
    anyhow::ensure!(claims.iss == IAP_ISSUER, "IAP assertion issuer is invalid");
    anyhow::ensure!(
        !claims.sub.is_empty() && claims.sub.len() <= 255,
        "IAP assertion subject is invalid"
    );
    anyhow::ensure!(
        claims.iat <= now.saturating_add(IAP_CLOCK_SKEW.as_secs()),
        "IAP assertion is not yet valid"
    );
    anyhow::ensure!(
        claims.exp > now.saturating_sub(IAP_CLOCK_SKEW.as_secs()),
        "IAP assertion is expired"
    );
    anyhow::ensure!(
        lifetime <= IAP_MAX_TOKEN_LIFETIME.as_secs(),
        "IAP assertion lifetime is too long"
    );
    Ok(IapIdentity {
        email: claims.email.clone(),
        expires_unix: i64::try_from(claims.exp)?,
    })
}

async fn fetch_keys(client: &Client) -> anyhow::Result<CachedKeys> {
    let response = client.get(IAP_JWKS_URL).send().await?;
    anyhow::ensure!(
        response.status().is_success(),
        "IAP signing-key endpoint returned {}",
        response.status()
    );
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IAP_JWKS_BYTES as u64)
    {
        anyhow::bail!("IAP signing-key response exceeds the configured bound");
    }
    let bytes = response.bytes().await?;
    anyhow::ensure!(
        bytes.len() <= MAX_IAP_JWKS_BYTES,
        "IAP signing-key response exceeds the configured bound"
    );
    let jwks: JwkSet = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(
        !jwks.keys.is_empty() && jwks.keys.len() <= MAX_IAP_KEYS,
        "IAP signing-key response has an invalid key count"
    );
    let mut by_id = BTreeMap::new();
    for jwk in jwks.keys {
        let kid = jwk
            .common
            .key_id
            .clone()
            .filter(|kid| valid_key_id(kid))
            .ok_or_else(|| anyhow::anyhow!("IAP signing key has an invalid key id"))?;
        let key = DecodingKey::from_jwk(&jwk)?;
        anyhow::ensure!(
            by_id.insert(kid, key).is_none(),
            "IAP signing-key response repeats a key id"
        );
    }
    let now = Instant::now();
    Ok(CachedKeys {
        by_id,
        expires_at: now + IAP_KEY_CACHE_LIFETIME,
        unknown_key_refresh_after: now + IAP_UNKNOWN_KEY_REFRESH_INTERVAL,
    })
}

fn valid_backend_service_audience(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("/projects/") else {
        return false;
    };
    let Some((project, backend)) = rest.split_once("/global/backendServices/") else {
        return false;
    };
    !project.is_empty()
        && !backend.is_empty()
        && project.bytes().all(|byte| byte.is_ascii_digit())
        && backend.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_assertion_envelope(assertion: &str) -> bool {
    !assertion.is_empty()
        && assertion.len() <= MAX_IAP_ASSERTION_BYTES
        && assertion.split('.').count() == 3
        && assertion
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_key_id(kid: &str) -> bool {
    !kid.is_empty() && kid.len() <= 128 && !kid.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUDIENCE: &str = "/projects/123456789/global/backendServices/987654321";

    fn claims(email: &str) -> IapClaims {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        IapClaims {
            aud: AUDIENCE.into(),
            email: email.into(),
            exp: now + 600,
            iat: now,
            iss: IAP_ISSUER.into(),
            sub: "accounts.google.com:105571299525019779033".into(),
        }
    }

    #[test]
    fn iap_audience_and_assertion_envelope_are_strictly_validated() {
        assert!(valid_backend_service_audience(AUDIENCE));
        assert!(!valid_backend_service_audience(
            "https://arcadia.example.invalid"
        ));
        assert!(valid_assertion_envelope("header.payload.signature"));
        assert!(!valid_assertion_envelope("header.payload"));
        assert!(!valid_assertion_envelope("header.payload.signature!"));
    }

    #[test]
    fn iap_claims_preserve_identity_and_require_the_configured_audience() {
        let identity = validate_claims(&claims("operator@example.com"), AUDIENCE).unwrap();
        assert_eq!(identity.email, "operator@example.com");
        assert!(validate_claims(&claims("another-operator@example.com"), AUDIENCE).is_ok());
        assert!(
            validate_claims(
                &claims("operator@example.com"),
                "/projects/1/global/backendServices/2"
            )
            .is_err()
        );
    }
}
