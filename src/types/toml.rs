use toml_edit::DocumentMut;

use super::Format;

#[derive(Clone, Copy, Default)]
pub(crate) struct Toml {}

impl Format for Toml {
    fn format(&self, input: &str) -> Result<String, String> {
        input
            .parse::<DocumentMut>()
            .map(|doc| doc.to_string())
            .map_err(|err| err.to_string())
    }
}
