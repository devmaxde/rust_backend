use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Form;
use chrono::Utc;
use proc_macros::build_errors;
use schemars::JsonSchema;
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use tracing::error;

use db::models::user;

use crate::docs::tags::ApiTags;
use crate::services::auth::jwt::create_jwt;
use crate::services::auth::password::verify_password;
use crate::state::AppState;
use axum::Json;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize, JsonSchema)]
pub struct LoginToken {
    pub access_token: String,
}

use crate::errors::ApiError;
use crate::errors::ErrorBuilder;
use crate::errors::GeneratedDocs;

build_errors!(
    LoginError,
    401 = { InvalidCredentials};
    500 = { Error2, Error3 }
);

pub async fn login(
    State(state): State<AppState>,
    Form(login_data): Form<LoginRequest>,
) -> impl IntoApiResponse {
    let login_id = login_data.username.to_lowercase();

    // Look up user by username or email (both lowercased)
    let db_user = user::Entity::find()
        .filter(
            Condition::any()
                .add(user::Column::Username.eq(&login_id))
                .add(user::Column::Email.eq(&login_id)),
        )
        .one(&state.conn)
        .await
        .expect("failed to connect to db");

    let Some(db_user) = db_user else {
        error!("Login failed: user not found for '{}'", login_id);
        return LoginError::invalid_credentials()
            .description("User not found")
            .into_response();
    };

    // Check if account is locked
    if let Some(locked_until) = db_user.locked_until {
        let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
        if locked_until > now {
            error!(
                "Login failed: account locked for '{}' until {}",
                login_id, locked_until
            );
            return LoginError::invalid_credentials()
                .description("Account locked")
                .into_response();
        }
    }

    let authenticated = db_user
        .password_hash
        .as_ref()
        .map(|hash| verify_password(&login_data.password, hash))
        .unwrap_or(false);

    if !authenticated {
        // Increment failed login count
        let new_count = db_user.failed_login_count + 1;
        let mut active: user::ActiveModel = db_user.into();
        active.failed_login_count = Set(new_count);
        // Lock after 10 failed attempts for 15 minutes
        if new_count >= 10 {
            let lock_until: chrono::DateTime<chrono::FixedOffset> =
                (Utc::now() + chrono::Duration::minutes(15)).into();
            active.locked_until = Set(Some(lock_until));
        }
        let _ = active.update(&state.conn).await;
        error!(
            "Login failed: wrong credentials for '{}' (attempt {})",
            login_id, new_count
        );
        return LoginError::invalid_credentials().into_response();
    }

    // Success: reset failed count, update last_login_at, issue JWT
    let user_id = db_user.id;
    let permission_level = db_user.permission_level.clone();
    let mut active: user::ActiveModel = db_user.into();
    active.failed_login_count = Set(0);
    active.locked_until = Set(None);
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    active.last_login_at = Set(Some(now));
    let _ = active.update(&state.conn).await;

    let token = create_jwt(user_id, &permission_level);

    Json(LoginToken {
        access_token: token,
    })
    .into_response()
}

pub fn login_docs(op: TransformOperation) -> TransformOperation {
    LoginError::generated_error_docs(
        op.description("Login")
            .id("login")
            .tag(ApiTags::Auth.into())
            .response::<200, Json<LoginToken>>(),
    )
}
