use sqlformat::{format, FormatOptions, QueryParams};

use super::Format;

#[derive(Clone, Copy, Default)]
pub(crate) struct Sql {}

impl Format for Sql {
    fn format(&self, input: &str) -> Result<String, String> {
        let opts = FormatOptions {
            uppercase: true,
            ..FormatOptions::default()
        };
        let mut out = format(input, &QueryParams::None, opts);
        out.push('\n');
        Ok(out)
    }
}
