pub use env_var::{EnvVar, bool_env_var, env_var};
use std::sync::LazyLock;

/// Whether the application is running in stateless mode (`APHRODY_STATELESS` or `ZED_STATELESS`).
/// When true, in-memory databases are used instead of persistent storage.
pub static ZED_STATELESS: LazyLock<bool> = LazyLock::new(|| brand::bool_env("STATELESS"));
