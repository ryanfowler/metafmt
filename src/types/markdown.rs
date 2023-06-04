use super::{json::Json, sql::Sql, toml::Toml, yaml::Yaml, Format};

use cmarkfmt::Formatter;

#[derive(Copy, Clone, Default)]
pub(crate) struct Markdown {
    json: Json,
    sql: Sql,
    toml: Toml,
    yaml: Yaml,
}

impl Format for Markdown {
    fn format(&self, input: &str) -> Result<String, String> {
        Ok(Formatter::default()
            .with_code_formatter(Some(&|lang, code| {
                match lang {
                    "json" | "jsonc" | "hjson" | "jwcc" => self.json.format(code),
                    "md" => self.format(code),
                    "sql" => self.sql.format(code),
                    "toml" => self.toml.format(code),
                    "yml" | "yaml" => self.yaml.format(code),
                    _ => return None,
                }
                .map(Some)
                .unwrap_or(None)
            }))
            .format_cmark(input))
    }
}
