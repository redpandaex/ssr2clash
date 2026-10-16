use crate::model::app::AppError;

#[derive(Debug, Clone)]
pub struct GroupDefinition {
    pub name: String,
    pub kind: String,
    pub selectors: Vec<String>,
    pub url: Option<String>,
    pub interval: Option<u64>,
    pub timeout: Option<u64>,
    pub tolerance: Option<u64>,
}

impl GroupDefinition {
    pub fn parse(line: &str) -> Result<Self, AppError> {
        let fields: Vec<&str> = line.split('`').map(str::trim).collect();
        let invalid = || {
            AppError::new(
                "InvalidPresetLine",
                "分组定义缺少名称、类型、匹配条件或测速参数",
            )
        };
        if fields.len() < 3 || fields[0].is_empty() {
            return Err(invalid());
        }
        let mut group = Self {
            name: fields[0].into(),
            kind: fields[1].into(),
            selectors: vec![],
            url: None,
            interval: None,
            timeout: None,
            tolerance: None,
        };
        match fields[1] {
            "select" => group.selectors = fields[2..].iter().map(|value| (*value).into()).collect(),
            "url-test" | "fallback" | "load-balance" => {
                if fields.len() < 5 {
                    return Err(invalid());
                }
                let end = fields.len();
                let url = fields[end - 2];
                if !url.starts_with("https://") && !url.starts_with("http://") {
                    return Err(invalid());
                }
                let numbers: Vec<&str> = fields[end - 1].split(',').collect();
                let parse = |index: usize| -> Result<Option<u64>, AppError> {
                    match numbers.get(index).filter(|value| !value.is_empty()) {
                        Some(value) => value.parse().map(Some).map_err(|_| invalid()),
                        None => Ok(None),
                    }
                };
                group.interval = parse(0)?;
                if group.interval.is_none_or(|value| value == 0) {
                    return Err(invalid());
                }
                group.timeout = parse(1)?
                    .map(|value| value.checked_mul(1000).ok_or_else(invalid))
                    .transpose()?;
                group.tolerance = parse(2)?;
                group.url = Some(url.into());
                group.selectors = fields[2..end - 2]
                    .iter()
                    .map(|value| (*value).into())
                    .collect();
            }
            _ => {
                return Err(AppError::new(
                    "UnsupportedPresetOption",
                    "不支持该策略组类型",
                ))
            }
        }
        Ok(group)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPreset {
    pub groups: Vec<GroupDefinition>,
    pub rules: Vec<String>,
}
