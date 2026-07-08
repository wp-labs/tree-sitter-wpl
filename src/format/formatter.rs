use super::tree::{
    byte_to_char_index, collect_separator_ranges, find_range_at, find_separator_block,
    infer_closing_indent,
};

pub fn format(content: &str) -> Result<String, WplFormatError> {
    WplFormatter::new().format(content)
}

pub fn format_with_indent(content: &str, indent: usize) -> Result<String, WplFormatError> {
    WplFormatter::with_indent(indent).format(content)
}

pub fn format_or_original(content: &str) -> String {
    WplFormatter::new().format_or_original(content)
}

pub struct WplFormatter {
    indent: usize,
}

impl Default for WplFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl WplFormatter {
    const RAW_FUNCS: &[&str] = &[
        "symbol",
        "f_chars_not_has",
        "f_chars_has",
        "kv",
        "f_chars_in",
    ];

    pub fn new() -> Self {
        Self { indent: 4 }
    }

    pub fn with_indent(indent: usize) -> Self {
        Self {
            indent: indent.max(1),
        }
    }

    pub fn format(&self, content: &str) -> Result<String, WplFormatError> {
        self.format_inner(content)
    }

    pub fn format_or_original(&self, content: &str) -> String {
        match self.format(content) {
            Ok(value) => value,
            Err(_) => content.to_string(),
        }
    }

    fn format_inner(&self, content: &str) -> Result<String, WplFormatError> {
        let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
        let (separator_ranges, ts_ok) = collect_separator_ranges(&normalized);
        let mut out = String::with_capacity(normalized.len() + 64);
        let mut byte_offsets = Vec::new();
        let chars: Vec<char> = normalized
            .char_indices()
            .map(|(idx, ch)| {
                byte_offsets.push(idx);
                ch
            })
            .collect();
        let input_len = normalized.len();

        let mut bracket_stack: Vec<(char, char)> = Vec::new();
        let mut i = 0usize;
        let mut indent = 0usize;
        let mut start_of_line = true;
        let mut line_no = 1usize;

        let bytes = normalized.as_bytes();
        while i < chars.len() {
            let byte_idx = byte_offsets[i];
            if let Some((range_start, range_end)) = find_range_at(&separator_ranges, byte_idx) {
                let slice_start = if byte_idx < range_start {
                    range_start
                } else {
                    byte_idx
                };
                let slice = &normalized[slice_start..range_end];
                self.write_indent_if_needed(start_of_line, indent, &mut out);
                out.push_str(slice);
                line_no = line_no.saturating_add(slice.matches('\n').count());
                start_of_line = slice.ends_with('\n');
                i = byte_to_char_index(&byte_offsets, range_end, input_len);
                continue;
            }

            let c = chars[i];
            let escaped = i > 0 && chars[i - 1] == '\\';

            if c == '"' {
                let next_non_ws = self.next_non_whitespace_pos(&chars, i + 1);
                let comma_follows = next_non_ws.is_some_and(|idx| chars[idx] == ',');
                let has_closing_quote = self.has_closing_quote(&chars, i + 1);
                if comma_follows || !has_closing_quote {
                    self.write_indent_if_needed(start_of_line, indent, &mut out);
                    out.push('"');
                    let new_i = next_non_ws.unwrap_or(i + 1);
                    line_no = line_no
                        .saturating_add(chars[i..new_i].iter().filter(|ch| **ch == '\n').count());
                    i = new_i;
                    start_of_line = false;
                    continue;
                }

                let (literal, consumed) = self.read_string(&chars[i..], line_no)?;
                self.write_indent_if_needed(start_of_line, indent, &mut out);
                out.push_str(&literal);
                line_no = line_no.saturating_add(literal.matches('\n').count());
                i += consumed;
                start_of_line = false;
                continue;
            }

            if c == 'r' && i + 1 < chars.len() && chars[i + 1] == '#' {
                let (literal, consumed) = self.read_raw_string(&chars[i..], line_no)?;
                self.write_indent_if_needed(start_of_line, indent, &mut out);
                out.push_str(&literal);
                line_no = line_no.saturating_add(literal.matches('\n').count());
                i += consumed;
                start_of_line = false;
                continue;
            }

            if c == '#' && i + 1 < chars.len() && chars[i + 1] == '[' {
                let (ann, consumed) = self.read_bracket_block(&chars[i..], '[', ']', line_no)?;
                self.write_indent_if_needed(start_of_line, indent, &mut out);
                out.push_str(
                    &ann.replace('\n', " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                out.push('\n');
                line_no = line_no.saturating_add(ann.matches('\n').count());
                i += consumed;
                start_of_line = true;
                continue;
            }

            if c == '<' {
                let (fmt_block, consumed) =
                    self.read_bracket_block(&chars[i..], '<', '>', line_no)?;
                self.write_indent_if_needed(start_of_line, indent, &mut out);
                out.push_str(&fmt_block);
                line_no = line_no.saturating_add(fmt_block.matches('\n').count());
                i += consumed;
                start_of_line = false;
                continue;
            }

            if c.is_whitespace() {
                if c == '\n' {
                    if !start_of_line {
                        out.push('\n');
                    }
                    start_of_line = true;
                    line_no = line_no.saturating_add(1);
                } else if !start_of_line {
                    out.push(' ');
                }
                i += 1;
                continue;
            }

            if let Some(name_len) = self.starts_with_raw_func(&chars, i, Self::RAW_FUNCS) {
                if let Some((block, consumed)) = self.read_raw_func_block(&chars[i..], name_len) {
                    self.write_indent_if_needed(start_of_line, indent, &mut out);
                    out.push_str(&block);
                    line_no = line_no.saturating_add(block.matches('\n').count());
                    start_of_line = false;
                    i += consumed;
                    continue;
                }
            }

            if escaped && (c == '(' || c == ')' || c == '{' || c == '}' || c == '|' || c == ',') {
                self.write_indent_if_needed(start_of_line, indent, &mut out);
                out.push(c);
                start_of_line = false;
                i += 1;
                continue;
            }

            match c {
                '{' => {
                    if let Some(end) = find_separator_block(bytes, byte_idx, &separator_ranges) {
                        let slice = &normalized[byte_idx..end];
                        self.write_indent_if_needed(start_of_line, indent, &mut out);
                        out.push_str(slice);
                        line_no = line_no.saturating_add(slice.matches('\n').count());
                        start_of_line = slice.ends_with('\n');
                        i = byte_to_char_index(&byte_offsets, end, input_len);
                        continue;
                    }
                    bracket_stack.push(('{', '}'));
                    self.write_indent_if_needed(start_of_line, indent, &mut out);
                    out.push('{');
                    out.push('\n');
                    indent += 1;
                    start_of_line = true;
                    i += 1;
                }
                '}' => {
                    if let Some((_, expected)) = bracket_stack.pop() {
                        if expected != '}' {
                            return Err(WplFormatError::MismatchedBracket {
                                expected,
                                found: '}',
                                line: line_no,
                            });
                        }
                    } else if ts_ok {
                        let inferred = infer_closing_indent(&out, self.indent);
                        indent = inferred;
                        if !start_of_line {
                            out.push('\n');
                        }
                        self.write_indent_if_needed(true, indent, &mut out);
                        out.push('}');
                        out.push('\n');
                        start_of_line = true;
                        i += 1;
                        continue;
                    } else {
                        return Err(WplFormatError::UnexpectedClosing {
                            close: '}',
                            line: line_no,
                        });
                    }

                    indent = indent.saturating_sub(1);
                    if !start_of_line {
                        out.push('\n');
                    }
                    self.write_indent_if_needed(true, indent, &mut out);
                    out.push('}');
                    out.push('\n');
                    start_of_line = true;
                    i += 1;
                }
                '(' => {
                    if let Some((inner, consumed)) = self.peek_block(&chars[i..], '(', ')') {
                        if !inner.contains(',') && !inner.contains('|') {
                            self.write_indent_if_needed(start_of_line, indent, &mut out);
                            out.push('(');
                            out.push_str(inner.trim());
                            out.push(')');
                            line_no = line_no.saturating_add(
                                chars[i..i + consumed]
                                    .iter()
                                    .filter(|ch| **ch == '\n')
                                    .count(),
                            );
                            start_of_line = false;
                            i += consumed;
                            continue;
                        }
                    }
                    bracket_stack.push(('(', ')'));
                    self.write_indent_if_needed(start_of_line, indent, &mut out);
                    out.push('(');
                    out.push('\n');
                    indent += 1;
                    start_of_line = true;
                    i += 1;
                }
                ')' => {
                    if let Some((_, expected)) = bracket_stack.pop() {
                        if expected != ')' {
                            return Err(WplFormatError::MismatchedBracket {
                                expected,
                                found: ')',
                                line: line_no,
                            });
                        }
                    } else if ts_ok {
                        let inferred = infer_closing_indent(&out, self.indent);
                        indent = inferred;
                        if !start_of_line {
                            out.push('\n');
                        }
                        self.write_indent_if_needed(true, indent, &mut out);
                        out.push(')');
                        start_of_line = false;
                        i += 1;
                        continue;
                    } else {
                        return Err(WplFormatError::UnexpectedClosing {
                            close: ')',
                            line: line_no,
                        });
                    }

                    indent = indent.saturating_sub(1);
                    if !start_of_line {
                        out.push('\n');
                    }
                    self.write_indent_if_needed(true, indent, &mut out);
                    out.push(')');
                    start_of_line = false;
                    i += 1;
                }
                ',' => {
                    out.push(',');
                    out.push('\n');
                    start_of_line = true;
                    i += 1;
                }
                '|' => {
                    self.write_indent_if_needed(start_of_line, indent, &mut out);
                    if !start_of_line && !matches!(out.chars().last(), Some(' ' | '\n')) {
                        out.push(' ');
                    }
                    out.push('|');
                    out.push(' ');
                    while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                        if chars[i + 1] == '\n' {
                            line_no = line_no.saturating_add(1);
                        }
                        i += 1;
                    }
                    start_of_line = false;
                    i += 1;
                }
                _ => {
                    self.write_indent_if_needed(start_of_line, indent, &mut out);
                    out.push(c);
                    start_of_line = false;
                    i += 1;
                }
            }
        }

        if let Some((open, close)) = bracket_stack.pop() {
            return Err(WplFormatError::UnclosedBracket {
                open,
                close,
                line: line_no,
            });
        }

        let mut final_out = String::new();
        let mut last_blank = false;
        for line in out.trim_end().lines() {
            let blank = line.trim().is_empty();
            if blank && last_blank {
                continue;
            }
            last_blank = blank;
            final_out.push_str(line.trim_end());
            final_out.push('\n');
        }

        while final_out.ends_with("\n\n\n") {
            final_out.pop();
        }

        Ok(final_out)
    }

    fn write_indent_if_needed(&self, start_of_line: bool, indent: usize, buf: &mut String) {
        if start_of_line {
            for _ in 0..indent {
                buf.push_str(&" ".repeat(self.indent));
            }
        }
    }

    fn read_string(
        &self,
        input: &[char],
        line_no: usize,
    ) -> Result<(String, usize), WplFormatError> {
        let mut out = String::new();
        let mut escaped = false;
        for (idx, ch) in input.iter().enumerate() {
            out.push(*ch);
            if escaped {
                escaped = false;
                continue;
            }
            if *ch == '\\' {
                escaped = true;
            } else if *ch == '"' && idx > 0 {
                return Ok((out, idx + 1));
            }
        }
        Err(WplFormatError::UnclosedString { line: line_no })
    }

    fn read_raw_string(
        &self,
        input: &[char],
        line_no: usize,
    ) -> Result<(String, usize), WplFormatError> {
        let mut out = String::new();
        let mut hash_count = 0usize;
        let mut idx = 0usize;

        if input.get(idx) != Some(&'r') {
            return Err(WplFormatError::InvalidRawStringStart { line: line_no });
        }
        out.push('r');
        idx += 1;

        while idx < input.len() && input[idx] == '#' {
            out.push('#');
            hash_count += 1;
            idx += 1;
        }
        if idx >= input.len() || input[idx] != '"' {
            return Err(WplFormatError::InvalidRawStringStart { line: line_no });
        }
        out.push('"');
        idx += 1;

        while idx < input.len() {
            let ch = input[idx];
            out.push(ch);
            if ch == '"' {
                let mut matched = true;
                for h in 0..hash_count {
                    if idx + 1 + h >= input.len() || input[idx + 1 + h] != '#' {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    for _ in 0..hash_count {
                        out.push('#');
                    }
                    return Ok((out, idx + 1 + hash_count));
                }
            }
            idx += 1;
        }
        Err(WplFormatError::UnclosedRawString { line: line_no })
    }

    fn read_bracket_block(
        &self,
        input: &[char],
        open: char,
        close: char,
        line_no: usize,
    ) -> Result<(String, usize), WplFormatError> {
        let mut out = String::new();
        let mut depth = 0usize;
        for (idx, ch) in input.iter().enumerate() {
            out.push(*ch);
            if *ch == open {
                depth += 1;
            } else if *ch == close {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok((out, idx + 1));
                }
            }
        }
        Err(WplFormatError::UnclosedBracket {
            open,
            close,
            line: line_no,
        })
    }

    fn peek_block(&self, input: &[char], open: char, close: char) -> Option<(String, usize)> {
        let mut out = String::new();
        let mut depth = 0usize;
        let mut escaped = false;
        let mut in_str = false;
        for (idx, ch) in input.iter().enumerate() {
            if escaped {
                out.push(*ch);
                escaped = false;
                continue;
            }
            match ch {
                '\\' => {
                    out.push(*ch);
                    escaped = true;
                }
                '"' => {
                    out.push(*ch);
                    in_str = !in_str;
                }
                _ if in_str => out.push(*ch),
                _ if *ch == open => {
                    depth += 1;
                    if depth == 1 {
                        continue;
                    }
                    out.push(*ch);
                }
                _ if *ch == close => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((out, idx + 1));
                    }
                    out.push(*ch);
                }
                _ => out.push(*ch),
            }
        }
        None
    }

    fn next_non_whitespace_pos(&self, input: &[char], start: usize) -> Option<usize> {
        input
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
    }

    fn has_closing_quote(&self, input: &[char], start: usize) -> bool {
        let mut escaped = false;
        for ch in input.iter().skip(start) {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => return true,
                _ => {}
            }
        }
        false
    }

    fn starts_with_raw_func(&self, input: &[char], idx: usize, names: &[&str]) -> Option<usize> {
        for name in names {
            let pat: Vec<char> = name.chars().chain(['(']).collect();
            if idx + pat.len() > input.len() {
                continue;
            }
            if input[idx..idx + pat.len()]
                .iter()
                .zip(pat.iter())
                .all(|(a, b)| a == b)
            {
                return Some(name.len());
            }
        }
        None
    }

    fn read_raw_func_block(&self, input: &[char], _name_len: usize) -> Option<(String, usize)> {
        let mut out = String::new();
        let mut depth = 0i32;
        let mut in_str = false;
        let mut escaped = false;

        for (idx, ch) in input.iter().enumerate() {
            out.push(*ch);
            if escaped {
                escaped = false;
                continue;
            }
            if *ch == '\\' {
                escaped = true;
                continue;
            }
            if *ch == '"' {
                in_str = !in_str;
                continue;
            }
            if in_str {
                continue;
            }
            if *ch == '(' {
                depth += 1;
            } else if *ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some((out, idx + 1));
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub enum WplFormatError {
    SyntaxError {
        line: usize,
        message: String,
    },
    UnclosedString {
        line: usize,
    },
    InvalidRawStringStart {
        line: usize,
    },
    UnclosedRawString {
        line: usize,
    },
    UnclosedBracket {
        open: char,
        close: char,
        line: usize,
    },
    MismatchedBracket {
        expected: char,
        found: char,
        line: usize,
    },
    UnexpectedClosing {
        close: char,
        line: usize,
    },
}

impl std::fmt::Display for WplFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WplFormatError::SyntaxError { line, message } => {
                write!(f, "line {}: {}", line, message)
            }
            WplFormatError::UnclosedString { line } => {
                write!(f, "line {}: string literal is not closed", line)
            }
            WplFormatError::InvalidRawStringStart { line } => {
                write!(f, "line {}: invalid raw string prefix", line)
            }
            WplFormatError::UnclosedRawString { line } => {
                write!(f, "line {}: raw string is not closed", line)
            }
            WplFormatError::UnclosedBracket { open, close, line } => {
                write!(f, "line {}: unclosed bracket: {} ... {}", line, open, close)
            }
            WplFormatError::MismatchedBracket {
                expected,
                found,
                line,
            } => {
                write!(
                    f,
                    "line {}: mismatched bracket, expected {}, found {}",
                    line, expected, found
                )
            }
            WplFormatError::UnexpectedClosing { close, line } => {
                write!(f, "line {}: unexpected closing bracket {}", line, close)
            }
        }
    }
}

impl std::error::Error for WplFormatError {}

#[cfg(test)]
mod tests {
    use super::{format, format_or_original, format_with_indent, WplFormatError};

    const NGINX_PARSE_WPL_INPUT: &str = r#"package nginx {
rule nginx {
(
    ip:sip,
    2*_,
    time:timestamp<[,]>,
    chars:request",
    digit:status,
    digit:size,
    chars:referer",
    http/agent",
    _,
)
}
}
"#;

    const NGINX_PARSE_WPL_EXPECTED: &str = r#"package nginx {
    rule nginx {
        (
            ip:sip,
            2*_,
            time:timestamp<[,]>,
            chars:request",
            digit:status,
            digit:size,
            chars:referer",
            http/agent",
            _,
        )
    }
}
"#;

    #[test]
    fn formats_nested_rule() {
        let input = r#"package demo { rule test { (chars:name,digit:age) } }"#;
        let expected = "\
package demo {
    rule test {
        (
            chars:name,
            digit:age
        )
    }
}
";
        assert_eq!(format(input).unwrap(), expected);
    }

    #[test]
    fn preserves_separator_blocks() {
        let input = r#"package demo { rule test { (chars:name{||}, digit:age) } }"#;
        let expected = "\
package demo {
    rule test {
        (
            chars:name{||},
            digit:age
        )
    }
}
";
        assert_eq!(format(input).unwrap(), expected);
    }

    #[test]
    fn supports_custom_indent_and_fallback() {
        let input = r#"package demo { rule test { (chars:name) } }"#;
        let formatted = format_with_indent(input, 2).unwrap();
        assert!(formatted.contains("\n  rule test {\n"));
        assert_eq!(format_or_original("package demo {"), "package demo {");
    }

    #[test]
    fn reports_syntax_errors() {
        let err = format("package demo {").unwrap_err();
        assert!(matches!(err, WplFormatError::UnclosedBracket { .. }));
    }

    #[test]
    fn formats_nginx_parse_sample() {
        assert_eq!(
            format(NGINX_PARSE_WPL_INPUT).unwrap(),
            NGINX_PARSE_WPL_EXPECTED
        );
    }
}
