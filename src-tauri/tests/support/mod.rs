use base64::{engine::general_purpose, Engine};

pub fn ssr_link(server: &str, remarks: &str, group: &str, padding: bool) -> String {
    let encode = |value: &str| {
        if padding {
            general_purpose::URL_SAFE.encode(value)
        } else {
            general_purpose::URL_SAFE_NO_PAD.encode(value)
        }
    };
    let body = format!(
        "{server}:8388:origin:aes-256-cfb:plain:{}/?remarks={}&group={}",
        encode("test-password"),
        encode(remarks),
        encode(group)
    );
    format!("ssr://{}", encode(&body))
}
