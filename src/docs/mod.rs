use std::sync::Arc;

use aide::axum::routing::get;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::{OAuth2Flow, OAuth2Flows, Server, Tag};
use aide::swagger::Swagger;
use aide::transform::TransformOpenApi;
use aide::{axum::routing::get_with, openapi::OpenApi, redoc::Redoc};
use axum::response::{Html, IntoResponse, Redirect};
use axum::{Extension, Json};

use crate::docs::tags::ApiTags;
use crate::state::AppState;

pub mod tags;

pub fn docs_routes(state: AppState) -> ApiRouter {
    aide::generate::infer_responses(true);

    #[cfg(debug_assertions)]
    let open_api_path = "/docs/openapi.json";

    #[cfg(not(debug_assertions))]
    let open_api_path = "/api/docs/openapi.json";

    let router = ApiRouter::new()
        .route("/docs", get(doc_redirect))
        .route("/openapi.json", get_with(serve_docs, |p| p.tag("docs")))
        .route(
            "/redoc",
            get(Redoc::new(open_api_path)
                .with_title("Backend")
                .axum_handler()),
        )
        .route(
            "/swagger",
            get(Swagger::new(open_api_path)
                .with_title("Backend")
                .axum_handler()),
        )
        .route("/", get(index))
        .with_state(state);
    aide::generate::infer_responses(false);
    router
}

async fn index() -> impl IntoApiResponse {
    let html = include_str!("static/index.html");
    Html(html).into_response()
}

async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api).into_response()
}

pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    let oauth2_flows = OAuth2Flows {
        password: Some(OAuth2Flow::Password {
            token_url: "/oauth2/login".to_string(),
            refresh_url: None,
            scopes: Default::default(),
        }),
        ..Default::default()
    };

    api.title("Backend")
        .summary("The Documentation for the Backend API.")
        .description(include_str!("static/README.md"))
        .tag(Tag {
            name: ApiTags::Auth.into(),
            description: Some("User Authentication".into()),
            ..Default::default()
        })
        .security_scheme(
            "ApiKey",
            aide::openapi::SecurityScheme::OAuth2 {
                flows: oauth2_flows,
                description: Some("".into()),
                extensions: Default::default(),
            },
        )
        .server(Server {
            url: "http://localhost:3000".to_string(),
            description: None,
            variables: Default::default(),
            extensions: Default::default(),
        })
        .server(Server {
            url: "https://api.example.com".to_string(),
            description: None,
            variables: Default::default(),
            extensions: Default::default(),
        })
}

pub async fn doc_redirect() -> impl IntoApiResponse {
    Redirect::temporary("/docs/").into_response()
}
