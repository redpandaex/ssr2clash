use crate::model::app::AppError;
use base64::{engine::general_purpose, Engine};

pub fn decode_base64_text(input: &str) -> Result<String, AppError> {
    let clean: String = input
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect();
    let unpadded = clean.trim_end_matches('=');
    let padding = clean.len() - unpadded.len();
    if padding > 2 || (padding > 0 && clean.len() % 4 != 0) || unpadded.contains('=') {
        return Err(AppError::new("InvalidBase64", "Base64 补位格式无效"));
    }
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(unpadded)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(unpadded))
        .map_err(|_| AppError::new("InvalidBase64", "Base64 内容无效"))?;
    String::from_utf8(decoded)
        .map_err(|_| AppError::new("InvalidUtf8", "订阅内容不是有效的 UTF-8 文本"))
}
