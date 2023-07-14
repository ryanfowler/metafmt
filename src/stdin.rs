use std::io::{self, Read, Write};

use crate::types::{json::Json, markdown::Markdown, sql::Sql, toml::Toml, yaml::Yaml, Format};

pub(crate) fn format(filetype: Option<String>) -> i32 {
    let Some(filetype) = filetype else {
        eprintln!("error: the '--stdin-filetype' flag must be provided");
        return 1;
    };

    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("error: {}", err);
        return 1;
    }

    match filetype.as_str() {
        "json" => format_file(&input, Json::default()),
        "md" | "markdown" => format_file(&input, Markdown::default()),
        "sql" => format_file(&input, Sql::default()),
        "toml" => format_file(&input, Toml::default()),
        "yaml" | "yml" => format_file(&input, Yaml::default()),
        format => {
            eprintln!("error: unknown format '{}'", format);
            1
        }
    }
}

fn format_file(input: &str, formatter: impl Format) -> i32 {
    let output = match formatter.format(input) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("error: {}", err);
            return 1;
        }
    };

    if let Err(err) = io::stdout().write_all(output.as_bytes()) {
        eprintln!("error: {}", err);
        return 1;
    }

    0
}
