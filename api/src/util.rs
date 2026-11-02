use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngExt;

pub(crate) fn random_urlsafe_string(length: usize) -> String {
    let mut bytes = vec![0u8; length];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
