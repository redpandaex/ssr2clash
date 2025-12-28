use crate::model::convert::ConvertConfig;
use crate::ssr::converter::{convert_ssr_to_clash_proxy, generate_clash_config};
use crate::ssr::filter::filter_nodes;
use crate::ssr::parser::parse_ssr_link;

#[tauri::command]
pub fn convert_ssr_to_clash(
    ssr_links: Vec<String>,
    config: ConvertConfig,
) -> Result<String, String> {
    // 1. 解析所有 SSR 链接
    let mut ssr_nodes = Vec::new();
    for link in ssr_links {
        match parse_ssr_link(&link) {
            Ok(node) => ssr_nodes.push(node),
            Err(e) => eprintln!("解析节点失败: {}", e),
        }
    }

    if ssr_nodes.is_empty() {
        return Err("没有有效的节点".to_string());
    }

    // 2. 过滤节点
    let filtered_nodes = filter_nodes(ssr_nodes, &config.filter_keywords, &config.exclude_keywords);

    if filtered_nodes.is_empty() {
        return Err("过滤后没有剩余节点".to_string());
    }

    // 3. 转换为 Clash 代理
    let clash_proxies: Vec<_> = filtered_nodes
        .iter()
        .map(convert_ssr_to_clash_proxy)
        .collect();

    // 4. 生成完整配置
    let clash_config = generate_clash_config(clash_proxies);

    // 5. 序列化为 YAML
    let yaml =
        serde_yaml::to_string(&clash_config).map_err(|e| format!("生成 YAML 失败: {}", e))?;

    Ok(yaml)
}

#[tauri::command]
pub fn parse_ssr_link_command(link: String) -> Result<String, String> {
    let node = parse_ssr_link(&link).map_err(|e| format!("解析失败: {}", e))?;

    serde_json::to_string(&node).map_err(|e| format!("序列化失败: {}", e))
}
