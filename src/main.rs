use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use aide::openapi::OpenApi;
use axum::response::IntoResponse;
use axum::{http::StatusCode, Extension};
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{debug, info, Level};

use db::migrations::{Migrator, MigratorTrait};
use state::AppState;

use crate::docs::{api_docs, docs_routes};
use crate::routers::main_router;

pub mod common;
pub mod docs;
pub mod errors;
pub mod routers;
pub mod services;
pub mod state;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    aide::generate::on_error(|error| {
        tracing::error!("{error}");
    });

    env_logger::builder().filter_level(LevelFilter::Info).init();

    aide::generate::extract_schemas(true);

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");

    let mut opt = ConnectOptions::new(db_url);
    opt.sqlx_logging(false)
        .sqlx_logging_level(LevelFilter::Info);

    info!("Connecting to database");
    let conn = Database::connect(opt)
        .await
        .expect("Failed to connect to database");
    Migrator::up(&conn, None).await.unwrap();
    info!("Connected to database");

    let state = AppState::new(conn);

    let mut api = OpenApi::default();

    let app = main_router()
        .nest_api_service("/docs", docs_routes(state.clone()))
        .finish_api_with(&mut api, api_docs)
        .layer(CorsLayer::permissive())
        .layer(Extension(Arc::new(api)))
        .layer(
            TraceLayer::new_for_http().make_span_with(
                DefaultMakeSpan::default()
                    .include_headers(false)
                    .level(Level::INFO),
            ),
        )
        .with_state(state);

    let app = app.fallback(handler_404);

    let listener;
    #[cfg(feature = "listenfd")]
    {
        use listenfd::ListenFd;

        let mut listenfd = ListenFd::from_env();
        listener = match listenfd.take_tcp_listener(0).unwrap() {
            Some(listener) => {
                listener.set_nonblocking(true).unwrap();
                TcpListener::from_std(listener).unwrap()
            }
            // otherwise fall back to local listening
            None => TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        };
    }
    #[cfg(not(feature = "listenfd"))]
    {
        listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    }

    debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
