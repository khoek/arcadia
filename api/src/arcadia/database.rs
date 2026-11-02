use super::{user_parent, validate_document_id};
use crate::firestore::{Db, deserialize_stored_document};
use anyhow::Context;
use firestore::{
    FirestoreConsistencySelector, FirestoreDocument, FirestoreTransactionOps,
    FirestoreWritePrecondition,
};
use serde::{Deserialize, Serialize, de::IgnoredAny};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    Raw,
    /// A one-time-password seed.  The concrete query handler expands this
    /// column into the current and next code periods.
    Otp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: &'static str,
    pub kind: ColumnType,
}

impl Column {
    pub const fn raw(name: &'static str) -> Self {
        Self {
            name,
            kind: ColumnType::Raw,
        }
    }

    pub const fn otp(name: &'static str) -> Self {
        Self {
            name,
            kind: ColumnType::Otp,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Schema {
    pub name: &'static str,
    pub columns: &'static [Column],
}

impl Schema {
    pub fn column(&self, name: &str) -> Option<Column> {
        self.columns
            .iter()
            .copied()
            .find(|column| column.name == name)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Database {
    pub name: &'static str,
    pub collection: &'static str,
    pub schema: Schema,
}

impl Database {
    pub fn column(&self, name: &str) -> Option<Column> {
        self.schema.column(name)
    }

    pub fn require_column(&self, name: &str, kind: ColumnType) -> anyhow::Result<Column> {
        let column = self
            .column(name)
            .ok_or_else(|| anyhow::anyhow!("{} database has no `{name}` column", self.name))?;
        anyhow::ensure!(
            column.kind == kind,
            "{} database column `{name}` has an incompatible type",
            self.name
        );
        Ok(column)
    }

    pub async fn list(&self, db: &Db, user_id: &str) -> anyhow::Result<Vec<DatabaseRecord>> {
        let parent = user_parent(db, user_id)?;
        let documents = db
            .inner()
            .fluent()
            .select()
            .from(self.collection)
            .parent(&parent)
            .query()
            .await?;
        let mut records = documents
            .iter()
            .map(deserialize_database_document)
            .collect::<anyhow::Result<Vec<_>>>()?;
        for record in &records {
            self.validate_record(record)?;
        }
        records.sort_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(records)
    }

    pub async fn get(
        &self,
        db: &Db,
        user_id: &str,
        id: &str,
    ) -> anyhow::Result<Option<DatabaseRecord>> {
        validate_document_id(id)?;
        let parent = user_parent(db, user_id)?;
        let record = db
            .inner()
            .fluent()
            .select()
            .by_id_in(self.collection)
            .parent(&parent)
            .one(id)
            .await?
            .map(|document| deserialize_database_document(&document))
            .transpose()?;
        if let Some(record) = &record {
            self.validate_record(record)?;
        }
        Ok(record)
    }

    pub async fn create(
        &self,
        db: &Db,
        user_id: &str,
        mut record: DatabaseRecord,
    ) -> anyhow::Result<DatabaseRecord> {
        self.validate_record(&record)?;
        let parent = user_parent(db, user_id)?;
        validate_document_id(&record.id)?;
        let mut transaction = db.inner().begin_transaction().await?;
        let transaction_db =
            db.inner()
                .clone_with_consistency_selector(FirestoreConsistencySelector::Transaction(
                    transaction.transaction_id().clone(),
                ));
        let documents = transaction_db
            .fluent()
            .select()
            .from(self.collection)
            .parent(&parent)
            .query()
            .await?;
        let existing = match documents
            .iter()
            .map(deserialize_database_document)
            .collect::<anyhow::Result<Vec<_>>>()
        {
            Ok(existing) => existing,
            Err(error) => {
                transaction
                    .rollback()
                    .await
                    .map_err(anyhow::Error::from)
                    .ok();
                return Err(error);
            }
        };
        for existing_record in &existing {
            if let Err(error) = self.validate_record(existing_record) {
                transaction
                    .rollback()
                    .await
                    .map_err(anyhow::Error::from)
                    .ok();
                return Err(error);
            }
        }
        let next_position = existing
            .into_iter()
            .map(|record| record.position)
            .max()
            .unwrap_or(-1)
            .checked_add(1)
            .context("record position overflow")?;
        record.position = next_position;
        transaction.update_object_at(
            &parent,
            self.collection,
            &record.id,
            &record,
            None,
            Some(FirestoreWritePrecondition::Exists(false)),
            vec![],
        )?;
        transaction.commit().await?;
        Ok(record)
    }

    pub async fn update(
        &self,
        db: &Db,
        user_id: &str,
        record: &DatabaseRecord,
    ) -> anyhow::Result<bool> {
        self.validate_record(record)?;
        validate_document_id(&record.id)?;
        let updated: Result<IgnoredAny, firestore::errors::FirestoreError> = db
            .inner()
            .fluent()
            .update()
            .in_col(self.collection)
            .precondition(FirestoreWritePrecondition::Exists(true))
            .document_id(&record.id)
            .parent(&user_parent(db, user_id)?)
            .object(record)
            .execute()
            .await;
        match updated {
            Ok(_) => Ok(true),
            Err(firestore::errors::FirestoreError::DataNotFoundError(_)) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub async fn delete(&self, db: &Db, user_id: &str, id: &str) -> anyhow::Result<bool> {
        let parent = user_parent(db, user_id)?;
        validate_document_id(id)?;
        let mut transaction = db.inner().begin_transaction().await?;
        let transaction_db =
            db.inner()
                .clone_with_consistency_selector(FirestoreConsistencySelector::Transaction(
                    transaction.transaction_id().clone(),
                ));
        match transaction_db
            .fluent()
            .select()
            .by_id_in(self.collection)
            .parent(&parent)
            .one(id)
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => {
                transaction.rollback().await?;
                return Ok(false);
            }
            Err(error) => {
                transaction.rollback().await.ok();
                return Err(error.into());
            }
        }
        transaction.delete_by_id_at(
            &parent,
            self.collection,
            id,
            Some(FirestoreWritePrecondition::Exists(true)),
        )?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn reorder(
        &self,
        db: &Db,
        user_id: &str,
        ids: &[String],
    ) -> Result<(), ReorderError> {
        let desired = ids.iter().collect::<BTreeSet<_>>();
        if desired.len() != ids.len() || ids.iter().any(|id| validate_document_id(id).is_err()) {
            return Err(ReorderError::InvalidIds);
        }
        let parent = user_parent(db, user_id).map_err(ReorderError::Internal)?;
        let mut transaction = db
            .inner()
            .begin_transaction()
            .await
            .map_err(anyhow::Error::from)?;
        let transaction_db =
            db.inner()
                .clone_with_consistency_selector(FirestoreConsistencySelector::Transaction(
                    transaction.transaction_id().clone(),
                ));
        let records = transaction_db
            .fluent()
            .select()
            .from(self.collection)
            .parent(&parent)
            .query()
            .await
            .map_err(anyhow::Error::from)?
            .iter()
            .map(deserialize_database_document)
            .collect::<anyhow::Result<Vec<_>>>()?;
        for record in &records {
            if let Err(error) = self.validate_record(record) {
                transaction
                    .rollback()
                    .await
                    .map_err(anyhow::Error::from)
                    .ok();
                return Err(ReorderError::Internal(error));
            }
        }
        let mut records = records
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect::<BTreeMap<_, _>>();
        if desired != records.keys().collect::<BTreeSet<_>>() {
            transaction.rollback().await.map_err(anyhow::Error::from)?;
            return Err(ReorderError::InvalidIds);
        }
        for (position, id) in ids.iter().enumerate() {
            let record = records
                .get_mut(id)
                .expect("validated reorder id must have a record");
            record.position = i64::try_from(position)
                .context("record position exceeds i64")
                .map_err(ReorderError::Internal)?;
            transaction
                .update_object_at(
                    &parent,
                    self.collection,
                    id,
                    record,
                    None,
                    Some(FirestoreWritePrecondition::Exists(true)),
                    vec![],
                )
                .map_err(anyhow::Error::from)
                .map_err(ReorderError::Internal)?;
        }
        transaction.commit().await.map_err(anyhow::Error::from)?;
        Ok(())
    }

    fn validate_record(&self, record: &DatabaseRecord) -> anyhow::Result<()> {
        validate_document_id(&record.id)?;
        for (name, value) in &record.columns {
            let Some(column) = self.column(name) else {
                anyhow::bail!(
                    "{} database (schema {}) contains unknown column `{name}`",
                    self.name,
                    self.schema.name,
                );
            };
            if column.kind == ColumnType::Otp {
                anyhow::ensure!(
                    value.is_string(),
                    "{} database OTP column `{name}` must contain a string seed",
                    self.name
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatabaseRecord {
    pub id: String,
    /// Presentation metadata shared by every arcadia record.  It is kept
    /// outside the declared application columns so schemas remain about data,
    /// not UI concerns.
    #[serde(default)]
    pub accent: String,
    #[serde(flatten)]
    pub columns: BTreeMap<String, Value>,
    pub position: i64,
    pub created_unix: i64,
    pub updated_unix: i64,
}

impl DatabaseRecord {
    pub fn new(
        id: String,
        columns: BTreeMap<String, Value>,
        accent: String,
        now_unix: i64,
    ) -> Self {
        Self {
            id,
            accent,
            columns,
            position: 0,
            created_unix: now_unix,
            updated_unix: now_unix,
        }
    }

    pub fn string(&self, column: &str) -> anyhow::Result<Option<&str>> {
        match self.columns.get(column) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value)),
            Some(value) => anyhow::bail!("column `{column}` is not a string: {value}"),
        }
    }

    pub fn required_string(&self, column: &str) -> anyhow::Result<&str> {
        self.string(column)?
            .ok_or_else(|| anyhow::anyhow!("column `{column}` is required"))
    }

    pub fn u64(&self, column: &str) -> anyhow::Result<Option<u64>> {
        match self.columns.get(column) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("column `{column}` is not an unsigned integer"))
                .map(Some),
            Some(value) => anyhow::bail!("column `{column}` is not an unsigned integer: {value}"),
        }
    }

    pub fn required_u64(&self, column: &str) -> anyhow::Result<u64> {
        self.u64(column)?
            .ok_or_else(|| anyhow::anyhow!("column `{column}` is required"))
    }
}

pub const AUTHENTICATOR_DATABASE: Database = Database {
    name: "authenticator",
    collection: "authenticators",
    schema: Schema {
        name: "authenticator-v1",
        columns: &[
            Column::raw("issuer"),
            Column::raw("account"),
            Column::raw("algorithm"),
            Column::raw("digits"),
            Column::raw("period_seconds"),
            Column::otp("secret"),
            Column::raw("icon_image"),
        ],
    },
};

pub const PASSWORDS_DATABASE: Database = Database {
    name: "passwords",
    collection: "vault_entries",
    schema: Schema {
        name: "passwords-v1",
        columns: &[
            Column::raw("name"),
            Column::raw("username"),
            Column::raw("password"),
            Column::raw("notes"),
            Column::raw("icon_image"),
        ],
    },
};

pub const CARDS_DATABASE: Database = Database {
    name: "cards",
    collection: "card_entries",
    schema: Schema {
        name: "cards-v1",
        columns: &[
            Column::raw("name"),
            Column::raw("number"),
            Column::raw("expiry"),
            Column::raw("cvc"),
            Column::raw("cardholder"),
            Column::raw("notes"),
            Column::raw("icon_image"),
        ],
    },
};

#[derive(Clone, Copy, Debug)]
pub struct ArcadiaDatabases {
    pub authenticator: Database,
    pub passwords: Database,
    pub cards: Database,
}

pub const ARCADIA_DATABASES: ArcadiaDatabases = ArcadiaDatabases {
    authenticator: AUTHENTICATOR_DATABASE,
    passwords: PASSWORDS_DATABASE,
    cards: CARDS_DATABASE,
};

#[derive(Debug, thiserror::Error)]
pub enum ReorderError {
    #[error("ordered ids must contain every current record exactly once")]
    InvalidIds,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

fn deserialize_database_document(document: &FirestoreDocument) -> anyhow::Result<DatabaseRecord> {
    let record = deserialize_stored_document::<DatabaseRecord>(document)?;
    anyhow::ensure!(
        document_id(&document.name) == Some(record.id.as_str()),
        "stored record id does not match its Firestore document id"
    );
    Ok(record)
}

fn document_id(name: &str) -> Option<&str> {
    name.rsplit('/').next().filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        ARCADIA_DATABASES, AUTHENTICATOR_DATABASE, CARDS_DATABASE, ColumnType, DatabaseRecord,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn arcadia_databases_have_explicit_column_schemas() {
        assert_eq!(ARCADIA_DATABASES.cards.name, "cards");
        assert_eq!(AUTHENTICATOR_DATABASE.schema.name, "authenticator-v1");
        assert_eq!(
            AUTHENTICATOR_DATABASE
                .column("secret")
                .map(|column| column.kind),
            Some(ColumnType::Otp)
        );
        assert_eq!(
            CARDS_DATABASE.column("number").map(|column| column.kind),
            Some(ColumnType::Raw)
        );
        assert!(
            AUTHENTICATOR_DATABASE
                .require_column("secret", ColumnType::Otp)
                .is_ok()
        );
        assert!(
            CARDS_DATABASE
                .require_column("number", ColumnType::Otp)
                .is_err()
        );
    }

    #[test]
    fn records_round_trip_flattened_columns() {
        let mut columns = BTreeMap::new();
        columns.insert("name".into(), json!("Example"));
        columns.insert("password".into(), json!("secret"));
        let record = DatabaseRecord::new("entry".into(), columns, "#246fca".into(), 1);
        let encoded = serde_json::to_value(&record).unwrap();
        assert_eq!(encoded["name"], "Example");
        assert_eq!(encoded["password"], "secret");
        let decoded: DatabaseRecord = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.required_string("password").unwrap(), "secret");
        assert_eq!(decoded.accent, "#246fca");
    }

    #[test]
    fn records_serialize_as_direct_firestore_fields() {
        let mut columns = BTreeMap::new();
        columns.insert("name".into(), json!("Example"));
        columns.insert("password".into(), json!("secret"));
        let record = DatabaseRecord::new("entry".into(), columns, "#246fca".into(), 1);
        let document = firestore::FirestoreDb::serialize_to_doc("arcadia/record", &record).unwrap();
        assert!(document.fields.contains_key("name"));
        assert!(document.fields.contains_key("password"));
        assert!(!document.fields.contains_key("columns"));
        let decoded =
            crate::firestore::deserialize_stored_document::<DatabaseRecord>(&document).unwrap();
        assert_eq!(decoded.required_string("password").unwrap(), "secret");
    }
}
