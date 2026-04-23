use serde::{Deserialize, Deserializer};

pub fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

#[macro_export]
macro_rules! update_db {
    ($model:ident, $body:ident, $( $field:ident ),* $(,)? ) => {{
        use sea_orm::Set;
        let mut updated = false;
        $(
            if let Some(value) = $body.$field.clone() {
                $model.$field = Set(value);
                updated = true;
            }
        )*
        updated
    }};
}

#[macro_export]
macro_rules! update_db_into {
    ($model:ident, $body:ident, $( $field:ident ),* $(,)? ) => {{
        use sea_orm::Set;
        let mut updated = false;
        $(
            if let Some(value) = $body.$field.clone() {
                $model.$field = Set(value.into());
                updated = true;
            }
        )*
        updated
    }};
}
