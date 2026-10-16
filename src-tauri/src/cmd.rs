use crate::{
    conversion::compile_subscription,
    model::{app::ProfileConfig, convert::ConvertConfig},
    preset::load_builtin_preset,
    ssr::parser::parse_ssr_link,
};

#[tauri::command]
pub fn convert_ssr_to_clash(
    ssr_links: Vec<String>,
    config: ConvertConfig,
) -> Result<String, String> {
    let profile = ProfileConfig {
        filter_keywords: config.filter_keywords,
        exclude_keywords: config.exclude_keywords,
        enable_udp: config.enable_udp,
        ..ProfileConfig::default()
    };
    let preset = load_builtin_preset().map_err(|error| error.to_string())?;
    compile_subscription(&ssr_links.join("\n"), &profile, &preset, 0)
        .map(|result| result.output.yaml)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn parse_ssr_link_command(link: String) -> Result<String, String> {
    let node = parse_ssr_link(&link).map_err(|error| format!("解析失败: {error}"))?;
    serde_json::to_string(&node).map_err(|_| "序列化节点失败".to_owned())
}
