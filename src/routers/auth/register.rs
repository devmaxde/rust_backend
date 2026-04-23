use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::extract::State;
use proc_macros::build_errors;
use schemars::JsonSchema;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use db::models::user;

use crate::docs::tags::ApiTags;
use crate::routers::auth::login::LoginToken;
use crate::services::auth::jwt::create_jwt;
use crate::services::auth::password::hash_password;
use crate::state::AppState;
use axum::Json;

#[derive(Debug, JsonSchema, Serialize, Deserialize)]
pub struct UserRegistrationRequest {
    first_name: String,
    last_name: String,
    email: String,
    username: String,
    password: String,
}

use crate::errors::ApiError;
use crate::errors::ErrorBuilder;
use crate::errors::GeneratedDocs;
use axum::response::IntoResponse;

build_errors!(
    RegisterError,
    400 = { PasswordTooWeak };
    409 = { UsernameTaken, EmailTaken };
    500 = { InternalError }
);

fn validate_password(password: &str) -> Result<(), String> {
    let mut missing = Vec::new();
    if password.len() < 8 {
        missing.push("at least 8 characters");
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        missing.push("an uppercase letter");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        missing.push("a lowercase letter");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        missing.push("a number");
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        missing.push("a special character");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("Password must contain: {}", missing.join(", ")))
    }
}

pub async fn register_fn(
    State(state): State<AppState>,
    Json(register_request): Json<UserRegistrationRequest>,
) -> impl IntoApiResponse {
    // Validate password strength
    if let Err(msg) = validate_password(&register_request.password) {
        return RegisterError::password_too_weak()
            .description(msg)
            .into_response();
    }

    // Check if username is already taken
    let existing = user::Entity::find()
        .filter(user::Column::Username.eq(&register_request.username))
        .one(&state.conn)
        .await
        .expect("failed to connect to db");

    if existing.is_some() {
        return RegisterError::username_taken().into_response();
    }

    // Check if email is already taken
    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&register_request.email))
        .one(&state.conn)
        .await
        .expect("failed to connect to db");

    if existing.is_some() {
        return RegisterError::email_taken().into_response();
    }

    let user_id = Uuid::new_v4();
    let (hash, salt, algo, params) = hash_password(&register_request.password);

    let db_user = user::ActiveModel {
        id: Set(user_id),
        first_name: Set(register_request.first_name),
        last_name: Set(register_request.last_name),
        email: Set(register_request.email),
        email_verified: Set(false),
        username: Set(register_request.username),
        permission_level: Set("User".to_string()),
        password_hash: Set(Some(hash)),
        password_salt: Set(Some(salt)),
        password_algo: Set(Some(algo)),
        password_params: Set(Some(params)),
        failed_login_count: Set(0),
        two_factor_enabled: Set(false),
        must_change_password: Set(false),
        ..Default::default()
    };

    if db_user.insert(&state.conn.clone()).await.is_err() {
        return RegisterError::internal_error().into_response();
    }

    let token = create_jwt(user_id, "User");

    Json(LoginToken {
        access_token: token,
    })
    .into_response()
}

pub fn register_docs(op: TransformOperation) -> TransformOperation {
    RegisterError::generated_error_docs(
        op.description("Register a new user")
            .response::<200, Json<LoginToken>>()
            .id("register")
            .tag(ApiTags::Auth.into()),
    )
}
