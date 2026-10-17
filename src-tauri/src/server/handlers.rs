use crate::{model::app::AppError, service::AppService};
use axum::{
    body::Body,
    extract::{ConnectInfo, Path, Request, State},
    http::{header, Response, StatusCode},
    middleware::{from_fn, Next},
    routing::get,
    Router,
};
use std::net::{IpAddr, SocketAddr};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
struct HttpState {
    service: Arc<AppService>,
    accepting: Arc<AtomicBool>,
}

pub(super) fn router(service: Arc<AppService>, accepting: Arc<AtomicBool>) -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async { ([(header::CACHE_CONTROL, "no-store")], "ok") }),
        )
        .route("/sub/{filename}", get(subscription).head(subscription_head))
        .layer(from_fn(local_peers_only))
        .with_state(HttpState { service, accepting })
}

async fn local_peers_only(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response<Body> {
    let allowed = match peer.ip() {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback(),
    };
    if !allowed {
        return text_response(StatusCode::FORBIDDEN, "仅支持本机或局域网访问");
    }
    next.run(request).await
}

async fn authorize(state: &HttpState, filename: &str) -> Result<(), Response<Body>> {
    if filename.strip_suffix(".yaml") != Some(state.service.subscription_token().await.as_str()) {
        return Err(text_response(StatusCode::NOT_FOUND, "未找到订阅"));
    }
    if !state.accepting.load(Ordering::Acquire) {
        return Err(error_response(AppError::new(
            "GenerationCancelled",
            "本地服务正在停止",
        )));
    }
    Ok(())
}

async fn subscription(
    State(state): State<HttpState>,
    Path(filename): Path<String>,
) -> Response<Body> {
    if let Err(response) = authorize(&state, &filename).await {
        return response;
    }
    match state.service.serve_subscription().await {
        Ok(result) => yaml_response(result.yaml, result.stale, false),
        Err(error) => error_response(error),
    }
}

async fn subscription_head(
    State(state): State<HttpState>,
    Path(filename): Path<String>,
) -> Response<Body> {
    if let Err(response) = authorize(&state, &filename).await {
        return response;
    }
    let status = state.service.status().await;
    match status.active_result {
        Some(output) => yaml_response(output.yaml, status.served_stale, true),
        None => text_response(StatusCode::BAD_GATEWAY, "尚未生成可用配置"),
    }
}

fn yaml_response(yaml: String, stale: bool, head_only: bool) -> Response<Body> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/yaml; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"subscription.yaml\"",
        )
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_LENGTH, yaml.len());
    if stale {
        builder = builder
            .header("x-ssr2clash-stale", "true")
            .header(header::WARNING, "110 - \"Response is stale\"");
    }
    builder
        .body(if head_only {
            Body::empty()
        } else {
            Body::from(yaml)
        })
        .expect("static valid HTTP headers")
}

fn text_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(message.to_owned()))
        .expect("static valid HTTP headers")
}

fn error_response(error: AppError) -> Response<Body> {
    let busy = matches!(
        error.code.as_str(),
        "TooManyRequests" | "GenerationCancelled"
    );
    let mut response = text_response(
        if busy {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::BAD_GATEWAY
        },
        &error.message,
    );
    if busy {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "1".parse().unwrap());
    }
    response
}
