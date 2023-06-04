use toml_edit::Document;

use super::Format;

#[derive(Clone, Copy, Default)]
pub(crate) struct Toml {}

impl Format for Toml {
    fn format(&self, input: &str) -> Result<String, String> {
        input
            .parse::<Document>()
            .map(|doc| doc.to_string())
            .map_err(|err| err.to_string())
    }
}
