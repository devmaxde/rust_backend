use aide::axum::ApiRouter;

use crate::state::AppState;

mod auth;

pub fn main_router() -> ApiRouter<AppState> {
    let mut router = ApiRouter::new();
    router = auth::register_auth_routes(router);
    router
}
