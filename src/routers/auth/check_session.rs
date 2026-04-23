use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;

use crate::docs::tags::ApiTags;
use crate::services::auth::user_auth::UserAuth;
use axum::Json;

pub async fn check_session(_claims: UserAuth) -> impl IntoApiResponse {
    Json(true)
}

pub fn check_session_docs(op: TransformOperation) -> TransformOperation {
    op.description("CheckSession")
        .id("checkSession")
        .tag(ApiTags::Auth.into())
        .security_requirement("ApiKey")
        .response::<204, Json<bool>>()
}
