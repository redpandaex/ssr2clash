pub mod builtin;
pub mod model;
pub mod validate;

pub use builtin::load_builtin_preset;
pub use validate::validate_config;

pub const BUILTIN_TARGETS: &[&str] = &[
    "DIRECT",
    "REJECT",
    "REJECT-DROP",
    "PASS",
    "COMPATIBLE",
    "GLOBAL",
];
