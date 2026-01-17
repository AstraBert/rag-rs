use crate::{embedding::embed_text, vectordb::VectorDB};
use async_openai::{Client, config::OpenAIConfig, types::responses::CreateResponseArgs};
use axum::http::header::CONTENT_TYPE;
use axum::http::method::Method;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};
use http::HeaderValue;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

use tracing::{debug, info, instrument};

const DEFAULT_PORT: u16 = 8000;
const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_RATE_LIMIT: u32 = 100;
const DEFAULT_SEARCH_LIMIT: u64 = 10;
const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1";

pub struct RagServer {
    qdrant_url: String,
    openai_api_key: String,
    pub collection_name: String,
    pub port: u16,
    pub host: IpAddr,
    pub rate_limit_per_minute: u32,
    pub cors: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct RagRequest {
    query: String,
    limit: Option<u64>,
    openai_model: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct RagResponse {
    response: String,
    retrieved: Vec<String>,
}

#[derive(Clone, Debug)]
struct AppState {
    vectordb: VectorDB,
    openai_client: Client<OpenAIConfig>,
}

#[derive(Deserialize, Serialize)]
struct RagError {
    status_code: usize,
    detail: String,
}

impl IntoResponse for RagError {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

impl RagResponse {
    fn new(response: String, retrieved: Vec<String>) -> Self {
        Self {
            response: response,
            retrieved: retrieved,
        }
    }
}

impl RagServer {
    pub fn new(
        qdrant_url: String,
        openai_api_key: Option<String>,
        collection_name: String,
        port: Option<u16>,
        host: Option<String>,
        rate_limit_per_minute: Option<u32>,
        cors: Option<String>,
    ) -> Self {
        let server_port = match port {
            Some(n) => n,
            None => DEFAULT_PORT,
        };
        let server_host = match host {
            Some(h) => {
                IpAddr::V4(Ipv4Addr::from_str(&h).expect("You should provide a valid IPv4 address"))
            }
            None => IpAddr::V4(
                Ipv4Addr::from_str(DEFAULT_HOST).expect("You should provide a valid IPv4 address"),
            ),
        };
        let server_rate_limit = match rate_limit_per_minute {
            Some(r) => r,
            None => DEFAULT_RATE_LIMIT,
        };
        let api_key = match openai_api_key {
            Some(a) => a,
            None => {
                let key = std::env::var("OPENAI_API_KEY").expect("If OpenAI API key is not provided as an argument, it should be set in the environment");
                key.to_string()
            }
        };
        Self {
            qdrant_url: qdrant_url,
            collection_name: collection_name,
            host: server_host,
            port: server_port,
            cors: cors,
            rate_limit_per_minute: server_rate_limit,
            openai_api_key: api_key,
        }
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        let vectordb = VectorDB::new(self.qdrant_url.clone(), self.collection_name.clone());
        let coll_loaded = vectordb.check_collection_ready().await?;
        if !coll_loaded {
            return Err(anyhow::anyhow!(
                "Vector database does not contain any vectors, or collection does not exist"
            ));
        }
        let state = AppState {
            vectordb: vectordb,
            openai_client: Client::with_config(
                OpenAIConfig::new().with_api_key(&self.openai_api_key),
            ),
        };
        let cors_layer = if self.cors.is_some()
            && let Some(cors) = &self.cors
        {
            CorsLayer::new()
                .allow_origin(
                    cors.parse::<HeaderValue>()
                        .expect("Should be able to parse URL into a header value."),
                )
                .allow_methods(vec![Method::POST])
                .allow_headers(vec![CONTENT_TYPE])
        } else {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Method::POST)
                .allow_headers(vec![CONTENT_TYPE])
        };
        let governor_conf = Box::new(
            GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(self.rate_limit_per_minute)
                .finish()
                .expect("Should be able to create a tower-governor config."),
        );
        let governor_limiter = governor_conf.limiter().clone();
        let interval = tokio::time::Duration::from_secs(60);
        // a separate background task to clean up
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(interval);
                if !governor_limiter.is_empty() {
                    debug!("rate limiting storage size: {}", governor_limiter.len());
                }
                governor_limiter.retain_recent();
            }
        });
        let governor_layer = GovernorLayer::new(governor_conf);
        let app = Router::new()
            .route("/queries", post(rag))
            .layer(governor_layer)
            .layer(cors_layer)
            .with_state(state);
        let addr = SocketAddr::from((self.host, self.port));
        tracing::info!("listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing_subscriber::fmt().pretty().init();
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }
}

#[instrument]
async fn rag(
    State(state): State<AppState>,
    Json(payload): Json<RagRequest>,
) -> Result<Json<RagResponse>, RagError> {
    let query_text = payload.query.clone();
    let embedding = embed_text(query_text);
    let search_limit = match payload.limit {
        Some(l) => l,
        None => DEFAULT_SEARCH_LIMIT,
    };
    let openai_model = match payload.openai_model {
        Some(m) => m,
        None => DEFAULT_OPENAI_MODEL.to_string(),
    };
    info!(event="RagSearchStart", data_id = %payload.query, "Starting vector search operation");
    let now = tokio::time::Instant::now();
    let results = match state.vectordb.search(embedding, search_limit).await {
        Ok(v) => v,
        Err(e) => {
            return Err(RagError {
                status_code: 500,
                detail: format!("Could not retrieve results because of {}", e.to_string()),
            });
        }
    };
    let elapsed = now.elapsed().as_millis();
    debug!(event="SearchResultsReport", data_id = %payload.query, "Total retrieved results: {}/{}", results.len(), search_limit);
    info!(event="RagSearchEnd", data_id = %payload.query, "Ended vector search operation in {} ms", elapsed);
    let context = &results.join("\n\n---\n\n");
    let request = CreateResponseArgs::default()
        .model(openai_model)
        .input(format!("Based on this context:\n\n```text\n{}\n```\n\n, reply to this query:\n\n```text\n{}\n```", context, payload.query))
        .build();
    info!(event="OpenAIResponseStart", data_id = %payload.query, "Starting OpenAI response generation");
    let now_resp = tokio::time::Instant::now();
    let openai_request = match request {
        Ok(r) => r,
        Err(e) => {
            return Err(RagError {
                status_code: 500,
                detail: format!(
                    "Could not generate an OpenAI request because of {}",
                    e.to_string()
                ),
            });
        }
    };
    let openai_response = state.openai_client.responses().create(openai_request).await;
    let response_text = match openai_response {
        Ok(r) => match r.output_text() {
            Some(s) => s,
            None => {
                return Err(RagError {
                    status_code: 500,
                    detail: "No response was generated by OpenAI".to_string(),
                });
            }
        },
        Err(e) => {
            return Err(RagError {
                status_code: 500,
                detail: format!(
                    "Could not generate an OpenAI response because of {}",
                    e.to_string()
                ),
            });
        }
    };
    let elapsed_resp = now_resp.elapsed().as_millis();
    info!(event="OpenAIResponseEnd", data_id = %payload.query, "Finished OpenAI response generation in {} ms", elapsed_resp);
    debug!(event="OverallLatencyReport", data_id = %payload.query, "Total latency: {} ms", elapsed + elapsed_resp);

    Ok(Json(RagResponse::new(response_text, results)))
}
