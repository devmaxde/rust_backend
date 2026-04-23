use std::fmt::Display;

use aide::OperationIo;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::IntoResponse,
    Json, RequestPartsExt,
};
use axum_extra::TypedHeader;
use headers::authorization::Bearer;
use headers::Authorization;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::services::auth::jwt::{verify_jwt, Claims};

#[derive(Debug, Serialize, Deserialize, OperationIo)]
pub struct UserAuth {
    pub(crate) token: String,
    #[serde(skip)]
    pub(crate) claims: Option<Claims>,
}

impl<S> FromRequestParts<S> for UserAuth
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = _parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let token = bearer.token().to_string();

        let claims = verify_jwt(&token).map_err(|_| AuthError::WrongCredentials)?;

        Ok(UserAuth {
            token,
            claims: Some(claims),
        })
    }
}

impl UserAuth {
    pub fn request_user_id(&self) -> Uuid {
        let claims = self.claims.as_ref().expect("Claims not set");
        Uuid::parse_str(&claims.sub).expect("sub is not a valid UUID")
    }

    pub fn permission_level(&self) -> &str {
        let claims = self.claims.as_ref().expect("Claims not set");
        &claims.permission_level
    }

    pub fn check_permission(&self, role: &str) -> bool {
        self.permission_level() == role
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl Display for UserAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token:{}", self.token)
    }
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
}
