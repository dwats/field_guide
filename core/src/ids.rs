//! Identifier newtypes. Every id is a UUID v7 string: time-ordered and unique without coordination.

use std::fmt;

use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// A fresh UUID v7.
            pub fn generate() -> Self {
                Self(uuid::Uuid::now_v7().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self(id)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self(id.to_owned())
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::Borrowed(ValueRef::Text(self.0.as_bytes())))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                String::column_result(value).map(Self)
            }
        }
    };
}

id_type!(
    /// Identifies a guide.
    GuideId
);
id_type!(
    /// Identifies a tag, in a guide's tree or in the global tree applied to guides.
    TagId
);
id_type!(
    /// Identifies a note.
    NoteId
);
id_type!(
    /// Identifies a media row. The file itself is content-addressed by its hash.
    MediaId
);
id_type!(
    /// Identifies a verb in a guide's vocabulary.
    VerbId
);
id_type!(
    /// Identifies a qualifier label in a guide's vocabulary.
    QualifierLabelId
);
id_type!(
    /// Identifies a relation: one stored edge between two notes.
    RelationId
);
id_type!(
    /// Identifies one qualifier on a relation.
    QualifierId
);
