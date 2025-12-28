use crate::model::clash::{ClashConfig, ClashProxy};
use crate::model::ssr::SsrNode;

/// 转换 SSR 节点为 Clash 代理
pub fn convert_ssr_to_clash_proxy(ssr: &SsrNode) -> ClashProxy {
    ClashProxy {
        name: ssr.remarks.clone(),
        proxy_type: "ssr".to_string(),
        server: ssr.server.clone(),
        port: ssr.server_port,
        cipher: ssr.method.clone(),
        password: ssr.password.clone(),
        protocol: ssr.protocol.clone(),
        obfs: ssr.obfs.clone(),
        obfs_param: if ssr.obfs_param.is_empty() {
            None
        } else {
            Some(ssr.obfs_param.clone())
        },
        protocol_param: if ssr.protocol_param.is_empty() {
            None
        } else {
            Some(ssr.protocol_param.clone())
        },
    }
}

/// 生成完整的 Clash 配置
pub fn generate_clash_config(proxies: Vec<ClashProxy>) -> ClashConfig {
    ClashConfig {
        port: 7890,
        socks_port: 7891,
        allow_lan: false,
        mode: "rule".to_string(),
        log_level: "info".to_string(),
        proxies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_ssr_to_clash_proxy() {
        let ssr = SsrNode {
            server: "example.com".to_string(),
            server_port: 8388,
            protocol: "origin".to_string(),
            method: "aes-256-cfb".to_string(),
            obfs: "plain".to_string(),
            password: "password123".to_string(),
            obfs_param: String::new(),
            protocol_param: String::new(),
            remarks: "测试节点".to_string(),
            group: String::new(),
        };

        let proxy = convert_ssr_to_clash_proxy(&ssr);

        assert_eq!(proxy.name, "测试节点");
        assert_eq!(proxy.proxy_type, "ssr");
        assert_eq!(proxy.server, "example.com");
        assert_eq!(proxy.port, 8388);
    }
}
