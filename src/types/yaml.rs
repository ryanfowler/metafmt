use super::Format;

#[derive(Clone, Copy, Default)]
pub(crate) struct Yaml {}

impl Format for Yaml {
    fn format(&self, input: &str) -> Result<String, String> {
        format_yaml(input)
    }
}

const INDENT_WIDTH: usize = 2;
const MAX_LINE_LENGTH: usize = 100;

fn format_yaml(input: &str) -> Result<String, String> {
    if input.is_empty() {
        return Ok(String::new());
    }

    // Validate YAML syntax using yaml-rust2.
    validate_yaml(input)?;

    // Normalize line endings to LF.
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");

    // Tokenize lines.
    let tokens = tokenize(&normalized);

    // Emit formatted output.
    let output = emit(&tokens);

    Ok(output)
}

fn validate_yaml(input: &str) -> Result<(), String> {
    yaml_rust2::YamlLoader::load_from_str(input)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

// --- Tokenizer ---

#[derive(Debug, Clone)]
enum Token {
    DocumentStart {
        trailing_comment: Option<String>,
    },
    DocumentEnd {
        trailing_comment: Option<String>,
    },
    Directive(String),
    Blank,
    Comment {
        indent: usize,
        text: String,
    },
    MappingKey {
        indent: usize,
        key: String,
        value: Option<String>,
        inline_comment: Option<String>,
        is_merge_tag: bool,
    },
    SequenceEntry {
        indent: usize,
        value: Option<String>,
        inline_comment: Option<String>,
    },
    BlockScalarHeader {
        indent: usize,
        header: String,
        inline_comment: Option<String>,
    },
    BlockScalarLine {
        text: String,
    },
    Continuation {
        indent: usize,
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Context {
    Normal,
    BlockScalar { parent_indent: usize },
}

fn measure_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn find_inline_comment(s: &str) -> Option<usize> {
    // Find a # that is preceded by whitespace and not inside quotes.
    let bytes = s.as_bytes();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' if !in_double_quote => in_single_quote = !in_single_quote,
            b'"' if !in_single_quote => in_double_quote = !in_double_quote,
            b'\\' if in_double_quote => {
                i += 1; // skip escaped char
            }
            b'#' if !in_single_quote && !in_double_quote => {
                if i > 0 && bytes[i - 1] == b' ' {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn tokenize(input: &str) -> Vec<Token> {
    let lines: Vec<&str> = input.split('\n').collect();
    let mut tokens = Vec::new();
    let mut context = Context::Normal;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Handle trailing empty line from final newline.
        if i == lines.len() - 1 && trimmed.is_empty() {
            break;
        }

        match context {
            Context::BlockScalar { parent_indent } => {
                if trimmed.is_empty() {
                    tokens.push(Token::BlockScalarLine {
                        text: String::new(),
                    });
                } else {
                    let line_indent = measure_indent(line);
                    if line_indent <= parent_indent && !trimmed.is_empty() {
                        // Exited block scalar context.
                        context = Context::Normal;
                        continue; // Re-process this line in normal context.
                    }
                    tokens.push(Token::BlockScalarLine {
                        text: line.to_string(),
                    });
                }
                i += 1;
                continue;
            }
            Context::Normal => {}
        }

        if trimmed.is_empty() {
            tokens.push(Token::Blank);
            i += 1;
            continue;
        }

        // Document start: ---
        if trimmed == "---" || trimmed.starts_with("--- ") || trimmed.starts_with("---\t") {
            let rest = trimmed.strip_prefix("---").unwrap().trim();
            let trailing_comment = if rest.starts_with('#') {
                Some(rest.to_string())
            } else if rest.is_empty() {
                None
            } else {
                // Something like "--- value", treat as trailing comment if it starts with #
                None
            };
            tokens.push(Token::DocumentStart { trailing_comment });
            i += 1;
            continue;
        }

        // Document end: ...
        if trimmed == "..." || trimmed.starts_with("... ") || trimmed.starts_with("...\t") {
            let rest = trimmed.strip_prefix("...").unwrap().trim();
            let trailing_comment = if rest.starts_with('#') {
                Some(rest.to_string())
            } else {
                None
            };
            tokens.push(Token::DocumentEnd { trailing_comment });
            i += 1;
            continue;
        }

        // Directive.
        if trimmed.starts_with('%') {
            tokens.push(Token::Directive(trimmed.to_string()));
            i += 1;
            continue;
        }

        // Comment line.
        if trimmed.starts_with('#') {
            let indent = measure_indent(line);
            tokens.push(Token::Comment {
                indent,
                text: trimmed.to_string(),
            });
            i += 1;
            continue;
        }

        let indent = measure_indent(line);

        // Sequence entry: starts with "- " or is exactly "-".
        if trimmed == "-" || trimmed.starts_with("- ") {
            let after_dash = if trimmed == "-" { "" } else { &trimmed[2..] };

            // Check if the value after "- " is a mapping key.
            // e.g. "- key: value" — we handle this as a sequence entry with the
            // full "key: value" as the value.
            let (value, inline_comment) = if after_dash.is_empty() {
                (None, None)
            } else if after_dash.starts_with('#') {
                (None, Some(after_dash.to_string()))
            } else {
                // Check for inline comment.
                match find_inline_comment(after_dash) {
                    Some(pos) => {
                        let val = after_dash[..pos].trim_end().to_string();
                        let comment = after_dash[pos..].to_string();
                        (if val.is_empty() { None } else { Some(val) }, Some(comment))
                    }
                    None => (Some(after_dash.to_string()), None),
                }
            };

            // Check if the value is a block scalar header.
            if let Some(ref val) = value {
                let val_trimmed = val.trim();
                if is_block_scalar_header(val_trimmed) {
                    // This is "- |" or "- >" etc.
                    // Parse it as a sequence entry with a block scalar.
                    tokens.push(Token::SequenceEntry {
                        indent,
                        value: None,
                        inline_comment: None,
                    });
                    let (header, comment) = split_block_scalar_header(val_trimmed);
                    tokens.push(Token::BlockScalarHeader {
                        indent: indent + 2,
                        header,
                        inline_comment: comment.or(inline_comment),
                    });
                    context = Context::BlockScalar {
                        parent_indent: indent + 2 - 1,
                    };
                    i += 1;
                    continue;
                }
            }

            // Check if the value itself contains a mapping key pattern.
            // e.g., "- key: value"
            if let Some(ref val) = value {
                if let Some(colon_pos) = find_mapping_colon(val) {
                    let key = val[..colon_pos].trim().to_string();
                    let after_colon = val[colon_pos + 1..].trim();

                    let is_merge = key == "<<";

                    let (map_value, map_comment) = if after_colon.is_empty() {
                        (None, inline_comment)
                    } else {
                        match find_inline_comment(after_colon) {
                            Some(pos) => {
                                let v = after_colon[..pos].trim_end().to_string();
                                let c = after_colon[pos..].to_string();
                                (if v.is_empty() { None } else { Some(v) }, Some(c))
                            }
                            None => (Some(after_colon.to_string()), inline_comment),
                        }
                    };

                    tokens.push(Token::SequenceEntry {
                        indent,
                        value: None,
                        inline_comment: None,
                    });
                    tokens.push(Token::MappingKey {
                        indent: indent + 2,
                        key,
                        value: map_value,
                        inline_comment: map_comment,
                        is_merge_tag: is_merge,
                    });

                    // Check if the mapping value is a block scalar header.
                    let bs_val = match tokens.last() {
                        Some(Token::MappingKey { value: Some(v), .. })
                            if is_block_scalar_header(v.trim()) =>
                        {
                            Some(v.clone())
                        }
                        _ => None,
                    };
                    if let Some(val) = bs_val {
                        let mk = tokens.pop().unwrap();
                        if let Token::MappingKey {
                            indent: mk_indent,
                            key,
                            inline_comment: mk_comment,
                            is_merge_tag,
                            ..
                        } = mk
                        {
                            let (header, hdr_comment) = split_block_scalar_header(val.trim());
                            tokens.push(Token::MappingKey {
                                indent: mk_indent,
                                key,
                                value: None,
                                inline_comment: None,
                                is_merge_tag,
                            });
                            tokens.push(Token::BlockScalarHeader {
                                indent: mk_indent + INDENT_WIDTH,
                                header,
                                inline_comment: hdr_comment.or(mk_comment),
                            });
                            context = Context::BlockScalar {
                                parent_indent: mk_indent + INDENT_WIDTH - 1,
                            };
                        }
                    }

                    i += 1;
                    continue;
                }
            }

            tokens.push(Token::SequenceEntry {
                indent,
                value,
                inline_comment,
            });

            // Check if after "- " is a block scalar header on its own
            // (Already handled above)

            i += 1;
            continue;
        }

        // Mapping key: contains unquoted ": " or ends with ":"
        if let Some(colon_pos) = find_mapping_colon(trimmed) {
            let key = trimmed[..colon_pos].trim().to_string();
            let after_colon = trimmed[colon_pos + 1..].trim();
            let is_merge = key == "<<";

            let (value, inline_comment) = if after_colon.is_empty() {
                (None, None)
            } else if after_colon.starts_with('#') {
                (None, Some(after_colon.to_string()))
            } else {
                match find_inline_comment(after_colon) {
                    Some(pos) => {
                        let val = after_colon[..pos].trim_end().to_string();
                        let comment = after_colon[pos..].to_string();
                        (if val.is_empty() { None } else { Some(val) }, Some(comment))
                    }
                    None => (Some(after_colon.to_string()), None),
                }
            };

            // Check if value is a block scalar header.
            if let Some(ref val) = value {
                let val_trimmed = val.trim();
                if is_block_scalar_header(val_trimmed) {
                    let (header, hdr_comment) = split_block_scalar_header(val_trimmed);
                    tokens.push(Token::MappingKey {
                        indent,
                        key,
                        value: None,
                        inline_comment: None,
                        is_merge_tag: is_merge,
                    });
                    tokens.push(Token::BlockScalarHeader {
                        indent: indent + INDENT_WIDTH,
                        header,
                        inline_comment: hdr_comment.or(inline_comment),
                    });
                    context = Context::BlockScalar {
                        parent_indent: indent + INDENT_WIDTH - 1,
                    };
                    i += 1;
                    continue;
                }
            }

            tokens.push(Token::MappingKey {
                indent,
                key,
                value,
                inline_comment,
                is_merge_tag: is_merge,
            });
            i += 1;
            continue;
        }

        // Continuation line (multiline scalar, etc.)
        tokens.push(Token::Continuation {
            indent,
            text: trimmed.to_string(),
        });
        i += 1;
    }

    tokens
}

fn is_block_scalar_header(s: &str) -> bool {
    let first = s.as_bytes().first();
    matches!(first, Some(b'|') | Some(b'>'))
}

fn split_block_scalar_header(s: &str) -> (String, Option<String>) {
    // Split "|-" or ">+" or "|2-" from trailing comment.
    match find_inline_comment(s) {
        Some(pos) => {
            let header = s[..pos].trim_end().to_string();
            let comment = s[pos..].to_string();
            (header, Some(comment))
        }
        None => (s.to_string(), None),
    }
}

fn find_mapping_colon(s: &str) -> Option<usize> {
    // Find a colon that indicates a mapping key.
    // Must be followed by a space, newline, or end of string.
    // Must not be inside quotes.
    let bytes = s.as_bytes();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' if !in_double_quote && brace_depth == 0 && bracket_depth == 0 => {
                in_single_quote = !in_single_quote
            }
            b'"' if !in_single_quote && brace_depth == 0 && bracket_depth == 0 => {
                in_double_quote = !in_double_quote
            }
            b'\\' if in_double_quote => {
                i += 1;
            }
            b'{' if !in_single_quote && !in_double_quote => brace_depth += 1,
            b'}' if !in_single_quote && !in_double_quote => brace_depth -= 1,
            b'[' if !in_single_quote && !in_double_quote => bracket_depth += 1,
            b']' if !in_single_quote && !in_double_quote => bracket_depth -= 1,
            b':' if !in_single_quote
                && !in_double_quote
                && brace_depth == 0
                && bracket_depth == 0 =>
            {
                // Colon must be followed by space, tab, or be at end.
                if i + 1 >= bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'\t' {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

// --- Emitter ---

struct IndentMapper {
    // Maps raw indent levels to nesting depths.
    stack: Vec<usize>,
}

impl IndentMapper {
    fn new() -> Self {
        IndentMapper { stack: vec![0] }
    }

    fn depth_for(&mut self, raw_indent: usize) -> usize {
        // Pop stack until we find a level <= raw_indent.
        while self.stack.len() > 1 && *self.stack.last().unwrap() > raw_indent {
            self.stack.pop();
        }

        if *self.stack.last().unwrap() == raw_indent {
            return self.stack.len() - 1;
        }

        // New deeper nesting level.
        self.stack.push(raw_indent);
        self.stack.len() - 1
    }

    fn reset(&mut self) {
        self.stack = vec![0];
    }
}

fn emit(tokens: &[Token]) -> String {
    let mut output = String::new();
    let mut mapper = IndentMapper::new();
    let mut prev_was_blank = false;
    let mut in_block_scalar = false;
    let mut block_scalar_base_indent: Option<usize> = 0.into();
    let mut block_scalar_canonical_indent: usize = 0;
    let mut skip_until_undent: Option<usize> = None;
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        // Handle merge tag skipping.
        if let Some(skip_indent) = skip_until_undent {
            match token {
                Token::MappingKey { indent, .. }
                | Token::SequenceEntry { indent, .. }
                | Token::Comment { indent, .. }
                | Token::Continuation { indent, .. }
                | Token::BlockScalarHeader { indent, .. } => {
                    if *indent > skip_indent {
                        i += 1;
                        continue;
                    }
                    skip_until_undent = None;
                }
                Token::BlockScalarLine { .. } => {
                    i += 1;
                    continue;
                }
                Token::Blank => {
                    i += 1;
                    continue;
                }
                _ => {
                    skip_until_undent = None;
                }
            }
        }

        match token {
            Token::DocumentStart { trailing_comment } => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("---");
                if let Some(comment) = trailing_comment {
                    output.push(' ');
                    output.push_str(comment);
                }
                output.push('\n');
                prev_was_blank = false;
                mapper.reset();
                in_block_scalar = false;
            }
            Token::DocumentEnd { trailing_comment } => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("...");
                if let Some(comment) = trailing_comment {
                    output.push(' ');
                    output.push_str(comment);
                }
                output.push('\n');
                prev_was_blank = false;
                mapper.reset();
                in_block_scalar = false;
            }
            Token::Directive(text) => {
                output.push_str(text);
                output.push('\n');
                prev_was_blank = false;
            }
            Token::Blank => {
                if in_block_scalar {
                    output.push('\n');
                } else if !prev_was_blank && !output.is_empty() {
                    output.push('\n');
                    prev_was_blank = true;
                }
            }
            Token::Comment { indent, text } => {
                if in_block_scalar {
                    in_block_scalar = false;
                }
                let depth = mapper.depth_for(*indent);
                let canonical_indent = depth * INDENT_WIDTH;
                write_indent(&mut output, canonical_indent);
                output.push_str(text);
                output.push('\n');
                prev_was_blank = false;
            }
            Token::MappingKey {
                indent,
                key,
                value,
                inline_comment,
                is_merge_tag,
            } => {
                if in_block_scalar {
                    in_block_scalar = false;
                }
                if *is_merge_tag {
                    skip_until_undent = Some(*indent);
                    i += 1;
                    continue;
                }

                let depth = mapper.depth_for(*indent);
                let canonical_indent = depth * INDENT_WIDTH;

                let mut line = String::new();
                write_indent(&mut line, canonical_indent);
                line.push_str(key);
                line.push(':');

                // Check if next token is a block scalar header (e.g., "key: |").
                if value.is_none() {
                    if let Some(Token::BlockScalarHeader {
                        header,
                        inline_comment: hdr_comment,
                        indent: hdr_indent,
                    }) = tokens.get(i + 1)
                    {
                        // Emit "key: |" on the same line.
                        line.push(' ');
                        line.push_str(header);
                        if let Some(comment) = hdr_comment.as_ref().or(inline_comment.as_ref()) {
                            line.push(' ');
                            line.push_str(comment);
                        }
                        output.push_str(&line);
                        output.push('\n');
                        in_block_scalar = true;
                        block_scalar_base_indent = None;
                        // The canonical indent for the block scalar content is
                        // based on the mapping key's indent.
                        block_scalar_canonical_indent = canonical_indent;
                        // Register the block scalar header's indent in the mapper.
                        mapper.depth_for(*hdr_indent);
                        i += 2; // Skip the block scalar header token.
                        prev_was_blank = false;
                        continue;
                    }
                }

                if let Some(val) = value {
                    let full_line = format!("{} {}", line, val);
                    if full_line.len() > MAX_LINE_LENGTH && can_break_value(val) {
                        if let Some(comment) = inline_comment {
                            line.push(' ');
                            line.push_str(comment);
                        }
                        output.push_str(&line);
                        output.push('\n');
                        write_indent(&mut output, canonical_indent + INDENT_WIDTH);
                        output.push_str(val);
                        output.push('\n');
                    } else {
                        line.push(' ');
                        line.push_str(val);
                        if let Some(comment) = inline_comment {
                            line.push(' ');
                            line.push_str(comment);
                        }
                        output.push_str(&line);
                        output.push('\n');
                    }
                } else {
                    if let Some(comment) = inline_comment {
                        line.push(' ');
                        line.push_str(comment);
                    }
                    output.push_str(&line);
                    output.push('\n');
                }
                prev_was_blank = false;
            }
            Token::SequenceEntry {
                indent,
                value,
                inline_comment,
            } => {
                if in_block_scalar {
                    in_block_scalar = false;
                }

                let depth = mapper.depth_for(*indent);
                let canonical_indent = depth * INDENT_WIDTH;

                let mut line = String::new();
                write_indent(&mut line, canonical_indent);
                line.push('-');

                if let Some(val) = value {
                    line.push(' ');
                    line.push_str(val);
                }
                if let Some(comment) = inline_comment {
                    line.push(' ');
                    line.push_str(comment);
                }

                // If this is a bare "- " followed by a mapping key, combine them
                // on the same line: "- key: value".
                if value.is_none() && inline_comment.is_none() {
                    if let Some(next) = tokens.get(i + 1) {
                        match next {
                            Token::MappingKey {
                                key,
                                value: mk_val,
                                inline_comment: mk_comment,
                                is_merge_tag,
                                indent: mk_indent,
                            } if !is_merge_tag => {
                                line.push(' ');
                                line.push_str(key);
                                line.push(':');

                                // Register the mapping key's indent in mapper.
                                mapper.depth_for(*mk_indent);

                                // Check if the mk_value is followed by a block scalar header.
                                if mk_val.is_none() {
                                    if let Some(Token::BlockScalarHeader {
                                        header,
                                        inline_comment: hdr_comment,
                                        indent: hdr_indent,
                                    }) = tokens.get(i + 2)
                                    {
                                        line.push(' ');
                                        line.push_str(header);
                                        if let Some(comment) =
                                            hdr_comment.as_ref().or(mk_comment.as_ref())
                                        {
                                            line.push(' ');
                                            line.push_str(comment);
                                        }
                                        output.push_str(&line);
                                        output.push('\n');
                                        in_block_scalar = true;
                                        block_scalar_base_indent = None;
                                        block_scalar_canonical_indent = canonical_indent;
                                        mapper.depth_for(*hdr_indent);
                                        i += 3;
                                        prev_was_blank = false;
                                        continue;
                                    }
                                }

                                if let Some(val) = mk_val {
                                    line.push(' ');
                                    line.push_str(val);
                                }
                                if let Some(comment) = mk_comment {
                                    line.push(' ');
                                    line.push_str(comment);
                                }
                                output.push_str(&line);
                                output.push('\n');
                                i += 2; // Skip the mapping key token.
                                prev_was_blank = false;
                                continue;
                            }
                            Token::BlockScalarHeader {
                                header,
                                inline_comment: hdr_comment,
                                indent: hdr_indent,
                            } => {
                                // "- |" on the same line.
                                line.push(' ');
                                line.push_str(header);
                                if let Some(comment) = hdr_comment {
                                    line.push(' ');
                                    line.push_str(comment);
                                }
                                output.push_str(&line);
                                output.push('\n');
                                in_block_scalar = true;
                                block_scalar_base_indent = None;
                                block_scalar_canonical_indent = canonical_indent;
                                mapper.depth_for(*hdr_indent);
                                i += 2;
                                prev_was_blank = false;
                                continue;
                            }
                            _ => {}
                        }
                    }
                }

                output.push_str(&line);
                output.push('\n');
                prev_was_blank = false;
            }
            Token::BlockScalarHeader {
                indent,
                header,
                inline_comment,
            } => {
                // This case handles standalone block scalar headers that weren't
                // consumed by the mapping key or sequence entry handling above.
                let depth = mapper.depth_for(*indent);
                let canonical_indent = depth * INDENT_WIDTH;

                write_indent(&mut output, canonical_indent);
                output.push_str(header);
                if let Some(comment) = inline_comment {
                    output.push(' ');
                    output.push_str(comment);
                }
                output.push('\n');

                in_block_scalar = true;
                block_scalar_base_indent = None;
                block_scalar_canonical_indent = canonical_indent;
                prev_was_blank = false;
            }
            Token::BlockScalarLine { text } => {
                if text.is_empty() {
                    output.push('\n');
                } else {
                    let raw_indent = measure_indent(text);
                    if block_scalar_base_indent.is_none() {
                        block_scalar_base_indent = Some(raw_indent);
                    }
                    let base = block_scalar_base_indent.unwrap_or(0);
                    let extra = raw_indent.saturating_sub(base);
                    let new_indent = block_scalar_canonical_indent + INDENT_WIDTH + extra;
                    write_indent(&mut output, new_indent);
                    output.push_str(text.trim_start());
                    output.push('\n');
                }
                prev_was_blank = false;
            }
            Token::Continuation { indent, text } => {
                if in_block_scalar {
                    in_block_scalar = false;
                }
                let depth = mapper.depth_for(*indent);
                let canonical_indent = depth * INDENT_WIDTH;
                write_indent(&mut output, canonical_indent);
                output.push_str(text);
                output.push('\n');
                prev_was_blank = false;
            }
        }

        i += 1;
    }

    // Ensure trailing newline.
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    // Remove trailing blank lines (keep just the final newline).
    while output.ends_with("\n\n") {
        output.pop();
    }

    output
}

fn write_indent(s: &mut String, indent: usize) {
    for _ in 0..indent {
        s.push(' ');
    }
}

fn can_break_value(val: &str) -> bool {
    // Don't break flow mappings/sequences or quoted strings.
    let first = val.as_bytes().first();
    !matches!(first, Some(b'{') | Some(b'[') | Some(b'"') | Some(b'\''))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert_eq!(format_yaml("").unwrap(), "");
    }

    #[test]
    fn test_simple_mapping() {
        let input = "key: value\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key: value\n");
    }

    #[test]
    fn test_nested_mapping() {
        let input = "parent:\n  child: value\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "parent:\n  child: value\n");
    }

    #[test]
    fn test_indent_normalization() {
        let input = "parent:\n    child: value\n    other: stuff\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "parent:\n  child: value\n  other: stuff\n");
    }

    #[test]
    fn test_sequence() {
        let input = "items:\n  - one\n  - two\n  - three\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "items:\n  - one\n  - two\n  - three\n");
    }

    #[test]
    fn test_comment_preservation() {
        let input = "# Top comment\nkey: value\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "# Top comment\nkey: value\n");
    }

    #[test]
    fn test_inline_comment() {
        let input = "key: value # inline comment\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key: value # inline comment\n");
    }

    #[test]
    fn test_blank_line_preservation() {
        let input = "key1: value1\n\nkey2: value2\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key1: value1\n\nkey2: value2\n");
    }

    #[test]
    fn test_blank_line_collapsing() {
        let input = "key1: value1\n\n\n\nkey2: value2\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key1: value1\n\nkey2: value2\n");
    }

    #[test]
    fn test_document_start() {
        let input = "---\nkey: value\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "---\nkey: value\n");
    }

    #[test]
    fn test_trailing_newline() {
        let input = "key: value";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key: value\n");
    }

    #[test]
    fn test_crlf_normalization() {
        let input = "key: value\r\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key: value\n");
    }

    #[test]
    fn test_block_literal_scalar() {
        let input = "text: |\n  line one\n  line two\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "text: |\n  line one\n  line two\n");
    }

    #[test]
    fn test_merge_tag_removal() {
        let input = "base: &base\n  key: value\nderived:\n  <<: *base\n  extra: stuff\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(
            result,
            "base: &base\n  key: value\nderived:\n  extra: stuff\n"
        );
    }

    #[test]
    fn test_invalid_yaml() {
        let input = "key: [\n";
        assert!(format_yaml(input).is_err());
    }

    #[test]
    fn test_sequence_of_mappings() {
        let input = "items:\n  - name: foo\n    value: bar\n  - name: baz\n    value: qux\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(
            result,
            "items:\n  - name: foo\n    value: bar\n  - name: baz\n    value: qux\n"
        );
    }

    #[test]
    fn test_flow_mapping() {
        let input = "key: {a: 1, b: 2}\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key: {a: 1, b: 2}\n");
    }

    #[test]
    fn test_flow_sequence() {
        let input = "key: [1, 2, 3]\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(result, "key: [1, 2, 3]\n");
    }

    #[test]
    fn test_anchor_alias() {
        let input = "defaults: &defaults\n  adapter: postgres\nproduction:\n  database: myapp\n  adapter: postgres\n";
        let result = format_yaml(input).unwrap();
        assert_eq!(
            result,
            "defaults: &defaults\n  adapter: postgres\nproduction:\n  database: myapp\n  adapter: postgres\n"
        );
    }
}
