use crate::model::app::{AppError, FetchProxy};
use std::time::Duration;
use url::Url;

pub const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;

pub fn parse_http_url(value: &str) -> Result<Url, AppError> {
    let url = Url::parse(value)
        .map_err(|_| AppError::new("InvalidSourceUrl", "请输入完整的 HTTP 或 HTTPS 地址"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::new(
            "UnsupportedResource",
            "只支持 HTTP 或 HTTPS 资源",
        ));
    }
    Ok(url)
}

#[derive(Clone)]
pub struct HttpFetcher {
    client: reqwest::Client,
}

impl HttpFetcher {
    pub fn new(proxy: FetchProxy) -> Result<Self, AppError> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(5))
            .retry(reqwest::retry::never())
            .user_agent(concat!("SSR2Clash/", env!("CARGO_PKG_VERSION")));
        if proxy == FetchProxy::Direct {
            builder = builder.no_proxy();
        }
        Ok(Self {
            client: builder
                .build()
                .map_err(|_| AppError::new("HttpClient", "无法初始化订阅获取客户端"))?,
        })
    }

    pub async fn get_text(&self, url: &str) -> Result<String, AppError> {
        let url = parse_http_url(url)?;
        let mut response = self.client.get(url).send().await.map_err(request_error)?;
        if !response.status().is_success() {
            return Err(AppError::new(
                "HttpStatus",
                format!("资源请求失败（HTTP {}）", response.status().as_u16()),
            ));
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_TEXT_BYTES as u64)
        {
            return Err(AppError::new("ResponseTooLarge", "资源内容超过 8 MiB 限制"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(request_error)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_TEXT_BYTES {
                return Err(AppError::new("ResponseTooLarge", "资源内容超过 8 MiB 限制"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| AppError::new("InvalidUtf8", "资源内容不是有效的 UTF-8 文本"))?;
        if text
            .trim()
            .trim_start_matches('\u{feff}')
            .trim_start()
            .starts_with('<')
        {
            return Err(AppError::new(
                "InvalidResponse",
                "服务器返回了网页，未获得有效订阅或规则",
            ));
        }
        Ok(text)
    }
}

fn request_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::new(
            "UpstreamTimeout",
            "资源获取超过 20 秒，请检查网络或代理设置",
        )
    } else {
        AppError::new(
            "UpstreamRequest",
            "资源获取失败，请检查地址、网络或代理设置",
        )
    }
}
