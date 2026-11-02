use crate::config::UserRecord;
use crate::firestore::{Db, load_user_by_email};

pub async fn load_arcadia_user(db: &Db, email: &str) -> anyhow::Result<Option<UserRecord>> {
    load_user_by_email(db, email).await
}
