use fjson::to_jsonc;

use super::Format;

#[derive(Clone, Copy, Default)]
pub(crate) struct Json {}

impl Format for Json {
    fn format(&self, input: &str) -> Result<String, String> {
        to_jsonc(input).map_err(|err| err.to_string())
    }
}
