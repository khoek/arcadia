use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub disabled: bool,
    pub session_version: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArcadiaSessionConfig {
    pub pepper: String,
    pub absolute_ttl_seconds: u64,
    pub idle_ttl_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArcadiaConfig {
    pub origin: Url,
    pub session: ArcadiaSessionConfig,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub arcadia: ArcadiaConfig,
}

impl AppConfig {
    pub fn require(self) -> anyhow::Result<Self> {
        if self.arcadia.origin.scheme() != "https"
            || self.arcadia.origin.host_str().is_none()
            || self.arcadia.origin.path() != "/"
            || self.arcadia.origin.query().is_some()
            || self.arcadia.origin.fragment().is_some()
        {
            anyhow::bail!(
                "arcadia.origin must be an HTTPS origin without a path, query, or fragment"
            );
        }
        if self.arcadia.session.pepper.len() < 32 {
            anyhow::bail!("arcadia.session.pepper must contain at least 32 characters");
        }
        if self.arcadia.session.idle_ttl_seconds == 0
            || self.arcadia.session.idle_ttl_seconds > self.arcadia.session.absolute_ttl_seconds
        {
            anyhow::bail!(
                "arcadia.session.idle_ttl_seconds must be > 0 and no greater than absolute_ttl_seconds"
            );
        }
        i64::try_from(self.arcadia.session.absolute_ttl_seconds)
            .map_err(|_| anyhow::anyhow!("arcadia.session.absolute_ttl_seconds exceeds i64"))?;
        i64::try_from(self.arcadia.session.idle_ttl_seconds)
            .map_err(|_| anyhow::anyhow!("arcadia.session.idle_ttl_seconds exceeds i64"))?;
        Ok(self)
    }
}
