use super::model::{GroupDefinition, ResolvedPreset};
use crate::model::app::AppError;
use std::sync::OnceLock;

pub fn load_builtin_preset() -> Result<ResolvedPreset, AppError> {
    static PRESET: OnceLock<Result<ResolvedPreset, AppError>> = OnceLock::new();
    PRESET
        .get_or_init(|| {
            let groups = include_str!("../../resources/presets/default/groups.txt")
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with(';') && !line.starts_with('#'))
                .map(GroupDefinition::parse)
                .collect::<Result<Vec<_>, _>>()?;
            let rules =
                serde_json::from_str(include_str!("../../resources/presets/default/rules.json"))
                    .map_err(|_| AppError::new("InvalidBuiltinPreset", "内置规则资源无效"))?;
            Ok(ResolvedPreset { groups, rules })
        })
        .clone()
}
