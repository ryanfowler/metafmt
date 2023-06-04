pub mod json;
pub mod markdown;
pub mod sql;
pub mod toml;
pub mod yaml;

pub(crate) trait Format {
    fn format(&self, input: &str) -> Result<String, String>;
}
