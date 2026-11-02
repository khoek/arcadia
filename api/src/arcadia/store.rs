use crate::config::UserRecord;
use crate::firestore::{Db, load_user_by_id};

pub async fn load_arcadia_user(db: &Db, user_id: &str) -> anyhow::Result<Option<UserRecord>> {
    load_user_by_id(db, user_id).await
}
