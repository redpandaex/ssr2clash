use crate::model::app::AppError;

#[derive(Debug)]
pub struct IniEntry {
    pub key: String,
    pub value: String,
    pub line: usize,
}

pub fn parse_ini(text: &str) -> Result<Vec<IniEntry>, AppError> {
    let mut entries = Vec::new();
    let mut section = String::new();
    for (index, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            at_line(
                AppError::new("InvalidPresetLine", "INI 配置必须使用 key=value 格式"),
                index + 1,
            )
        })?;
        if value.trim().is_empty() {
            continue;
        }
        if !matches!(
            section.as_str(),
            "" | "custom" | "proxy_groups" | "rulesets"
        ) {
            return Err(at_line(
                AppError::new("UnsupportedPresetOption", "不支持该 INI 配置区段"),
                index + 1,
            ));
        }
        let mut key = key.trim().to_ascii_lowercase();
        if section == "rulesets" && key == "enabled" {
            key = "enable_rule_generator".into();
        }
        if !matches!(
            key.as_str(),
            "custom_proxy_group"
                | "ruleset"
                | "enable_rule_generator"
                | "overwrite_original_rules"
                | "clash_rule_base"
        ) {
            return Err(at_line(
                AppError::new(
                    "UnsupportedPresetOption",
                    "该配置项尚不支持，请使用常用分组／规则配置",
                ),
                index + 1,
            ));
        }
        entries.push(IniEntry {
            key,
            value: value.trim().to_owned(),
            line: index + 1,
        });
    }
    Ok(entries)
}

pub fn at_line(error: AppError, line: usize) -> AppError {
    AppError::new(&error.code, format!("配置第 {line} 行：{}", error.message))
}
pub fn parse_bool(value: &str, line: usize) -> Result<bool, AppError> {
    match value.to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(at_line(
            AppError::new("InvalidPresetLine", "开关必须为 true 或 false"),
            line,
        )),
    }
}
