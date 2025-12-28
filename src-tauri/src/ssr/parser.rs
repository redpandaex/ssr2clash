use crate::model::ssr::SsrNode;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};

/// 解析 SSR 链接
///
/// 格式: ssr://base64(server:port:protocol:method:obfs:password_base64/?params)
pub fn parse_ssr_link(link: &str) -> Result<SsrNode> {
    // 1. 移除 "ssr://" 前缀
    let encoded = link
        .strip_prefix("ssr://")
        .ok_or_else(|| anyhow!("Invalid SSR link: missing ssr:// prefix"))?;

    // 2. Base64 解码
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;

    let decoded_str =
        String::from_utf8(decoded).map_err(|e| anyhow!("UTF-8 decode failed: {}", e))?;

    // 3. 分割主体和参数部分
    let parts: Vec<&str> = decoded_str.splitn(2, "/?").collect();
    let main_part = parts[0];
    let params_part = if parts.len() > 1 { parts[1] } else { "" };

    // 4. 解析主体部分: server:port:protocol:method:obfs:password_base64
    let main_parts: Vec<&str> = main_part.split(':').collect();
    if main_parts.len() != 6 {
        return Err(anyhow!(
            "Invalid SSR format: expected 6 parts, got {}",
            main_parts.len()
        ));
    }

    // 5. 解码密码
    let password_decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(main_parts[5])
        .map_err(|e| anyhow!("Password decode failed: {}", e))?;
    let password = String::from_utf8(password_decoded)
        .map_err(|e| anyhow!("Password UTF-8 decode failed: {}", e))?;

    // 6. 解析参数
    let mut obfs_param = String::new();
    let mut protocol_param = String::new();
    let mut remarks = String::new();
    let mut group = String::new();

    for param in params_part.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            let decoded_value = general_purpose::URL_SAFE_NO_PAD
                .decode(value)
                .ok()
                .and_then(|v| String::from_utf8(v).ok())
                .unwrap_or_default();

            match key {
                "obfsparam" => obfs_param = decoded_value,
                "protoparam" => protocol_param = decoded_value,
                "remarks" => remarks = decoded_value,
                "group" => group = decoded_value,
                _ => {}
            }
        }
    }

    // 7. 构建 SsrNode
    Ok(SsrNode {
        server: main_parts[0].to_string(),
        server_port: main_parts[1]
            .parse()
            .map_err(|e| anyhow!("Invalid port: {}", e))?,
        protocol: main_parts[2].to_string(),
        method: main_parts[3].to_string(),
        obfs: main_parts[4].to_string(),
        password,
        obfs_param,
        protocol_param,
        remarks,
        group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssr_link() {
        // 这里可以添加测试用例
        // let link = "ssr://...";
        // let result = parse_ssr_link(link);
        // assert!(result.is_ok());
    }
}
