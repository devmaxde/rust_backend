use crate::docs::tags::ApiTags;
use crate::services::auth::user_auth::UserAuth;
use aide::axum::IntoApiResponse;
use aide::transform::TransformOperation;
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn logout(_claims: UserAuth) -> impl IntoApiResponse {
    StatusCode::OK.into_response()
}

pub fn logout_docs(op: TransformOperation) -> TransformOperation {
    op.description("LogoutRequest")
        .id("logout")
        .tag(ApiTags::Auth.into())
        .security_requirement("ApiKey")
        .response::<204, ()>()
}
