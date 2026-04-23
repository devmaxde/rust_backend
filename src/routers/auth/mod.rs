use aide::axum::routing::{get_with, post_with};
use aide::axum::ApiRouter;

use crate::routers::auth::check_session::{check_session, check_session_docs};
use crate::routers::auth::login::{login, login_docs};
use crate::routers::auth::logout::{logout, logout_docs};
use crate::routers::auth::register::{register_docs, register_fn};
use crate::state::AppState;

mod check_session;
mod login;
mod logout;
mod register;

pub fn register_auth_routes(router: ApiRouter<AppState>) -> ApiRouter<AppState> {
    router
        .api_route("/oauth2/login", post_with(login, login_docs))
        .api_route("/oauth2/logout", post_with(logout, logout_docs))
        .api_route("/oauth2/register", post_with(register_fn, register_docs))
        .api_route(
            "/oauth2/check_session",
            get_with(check_session, check_session_docs),
        )
}
