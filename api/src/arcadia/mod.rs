//! Arcadia-owned persistence and data-model primitives.
//!
//! Keeping these pieces behind one outer module makes it clear that the
//! arcadia API owns the records without leaking storage details into the
//! rest of the service.

mod database;
mod store;

use crate::firestore::{Db, arcadia_parent};

#[allow(unused_imports)]
pub(crate) use database::{
    ARCADIA_DATABASES, AUTHENTICATOR_DATABASE, ArcadiaDatabases, CARDS_DATABASE, Column,
    ColumnType, Database, DatabaseRecord, PASSWORDS_DATABASE, ReorderError, Schema,
};
pub(crate) use store::load_arcadia_user;

const USERS_COLLECTION: &str = "users";

pub(crate) fn user_parent(db: &Db, user_id: &str) -> anyhow::Result<String> {
    validate_document_id(user_id)?;
    Ok(format!(
        "{}/{USERS_COLLECTION}/{user_id}",
        arcadia_parent(db.inner())?
    ))
}

pub(crate) fn validate_document_id(id: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !id.is_empty() && id.len() <= 128 && !id.contains('/'),
        "invalid Firestore document id"
    );
    Ok(())
}
