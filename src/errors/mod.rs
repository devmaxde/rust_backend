// src/errors.rs
use aide::{axum::IntoApiResponse, transform::TransformOperation};
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

pub trait ApiError {
    fn status_code(&self) -> StatusCode;
    fn message(&self) -> &'static str;
}

pub trait GeneratedDocs {
    fn generated_error_docs(op: TransformOperation) -> TransformOperation;
}

#[derive(Debug, Serialize, Clone)]
pub struct AppError<E: ApiError + Serialize> {
    pub error: String,
    pub error_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<Value>,
    #[serde(skip)]
    pub status: StatusCode,
    pub kind: E,
}

impl<E: ApiError + Serialize> AppError<E> {
    pub fn from_kind(kind: E) -> Self {
        Self {
            error: kind.message().to_string(),
            error_id: Uuid::new_v4(),
            error_details: None,
            status: kind.status_code(),
            kind,
        }
    }
    pub fn with_details(mut self, details: Value) -> Self {
        self.error_details = Some(details);
        self
    }
    pub fn with_details_opt(mut self, details: Option<Value>) -> Self {
        self.error_details = details;
        self
    }
    pub fn error_docs(op: TransformOperation) -> TransformOperation
    where
        E: GeneratedDocs,
    {
        E::generated_error_docs(op)
    }
    pub fn build_response(self) -> impl IntoApiResponse {
        (self.status, Json(self))
    }
}

impl<E: ApiError + Serialize> IntoResponse for AppError<E> {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self)).into_response()
    }
}

pub struct ErrorBuilder<E: ApiError + Serialize> {
    kind: E,
    details: Option<Value>,
}

impl<E: ApiError + Serialize> ErrorBuilder<E> {
    pub fn new(kind: E) -> Self {
        Self {
            kind,
            details: None,
        }
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.details = Some(serde_json::json!({ "description": text.into() }));
        self
    }
    pub fn details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
    pub fn build(self) -> AppError<E> {
        AppError::from_kind(self.kind).with_details_opt(self.details)
    }
    pub fn into_response(self) -> axum::response::Response {
        self.build().into_response()
    }
}
