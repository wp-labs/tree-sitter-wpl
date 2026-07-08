use tree_sitter::{Node, Parser};

pub(crate) fn collect_separator_ranges(input: &str) -> (Vec<(usize, usize)>, bool) {
    let mut parser = Parser::new();
    if parser.set_language(&crate::language()).is_err() {
        return (Vec::new(), false);
    }
    let tree = match parser.parse(input, None) {
        Some(value) => value,
        None => {
            return (Vec::new(), false);
        }
    };
    let root = tree.root_node();
    let mut ranges = Vec::new();

    fn visit(node: Node, ranges: &mut Vec<(usize, usize)>, input: &str) {
        let kind = node.kind();
        if matches!(kind, "pattern_sep" | "shortcut_sep") {
            let range = expand_separator_range(node.byte_range(), input);
            ranges.push(range);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            visit(child, ranges, input);
        }
    }

    visit(root, &mut ranges, input);
    collect_pattern_sep_blocks(input, root, &mut ranges);
    ranges.sort_unstable();
    ranges.dedup();
    (ranges, true)
}

fn expand_separator_range(range: std::ops::Range<usize>, input: &str) -> (usize, usize) {
    let bytes = input.as_bytes();
    let mut start = range.start;
    let mut end = range.end;
    if start > 0 && bytes.get(start.saturating_sub(1)) == Some(&b'{') {
        start = start.saturating_sub(1);
    }
    if end < bytes.len() && bytes.get(end) == Some(&b'}') {
        end = end.saturating_add(1);
    }
    (start, end)
}

fn collect_pattern_sep_blocks(input: &str, root: Node, ranges: &mut Vec<(usize, usize)>) {
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        while end < bytes.len() && bytes[end] != b'}' {
            end += 1;
        }
        if end >= bytes.len() || bytes[end] != b'}' {
            i += 1;
            continue;
        }
        let inner_start = i.saturating_add(1);
        let inner_end = end;
        if let Some(node) = root.descendant_for_byte_range(inner_start, inner_end) {
            if has_separator_ancestor(node) {
                ranges.push((i, end.saturating_add(1)));
            }
        }

        i = end.saturating_add(1);
    }
}

fn has_separator_ancestor(node: Node) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        let kind = n.kind();
        if matches!(kind, "separator" | "pattern_sep") {
            return true;
        }
        current = n.parent();
    }
    false
}

pub(crate) fn find_range_at(ranges: &[(usize, usize)], offset: usize) -> Option<(usize, usize)> {
    if ranges.is_empty() {
        return None;
    }
    match ranges.binary_search_by_key(&offset, |(s, _)| *s) {
        Ok(idx) => Some(ranges[idx]),
        Err(0) => None,
        Err(idx) => {
            let (start, end) = ranges[idx - 1];
            if offset >= start && offset < end {
                Some((start, end))
            } else {
                None
            }
        }
    }
}

pub(crate) fn find_separator_block(
    bytes: &[u8],
    start: usize,
    ranges: &[(usize, usize)],
) -> Option<usize> {
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    ranges.iter().find(|(s, _)| *s == start).map(|(_, e)| *e)
}

pub(crate) fn byte_to_char_index(offsets: &[usize], end: usize, input_len: usize) -> usize {
    if end >= input_len {
        return offsets.len();
    }
    match offsets.binary_search(&end) {
        Ok(idx) => idx,
        Err(idx) => idx,
    }
}

pub(crate) fn infer_closing_indent(out: &str, indent_unit: usize) -> usize {
    let line = out
        .rsplit('\n')
        .find(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return false;
            }
            !trimmed.chars().all(|ch| matches!(ch, ')' | '}' | ']'))
        })
        .unwrap_or("");
    let leading = line.chars().take_while(|ch| *ch == ' ').count();
    let trimmed = line.trim_end();
    let ends_with_open = trimmed.ends_with('{') || trimmed.ends_with('(') || trimmed.ends_with('[');
    let is_annotation = trimmed.starts_with("#[");
    if ends_with_open || is_annotation {
        return leading / indent_unit;
    }
    if leading >= indent_unit {
        leading / indent_unit - 1
    } else {
        0
    }
}
