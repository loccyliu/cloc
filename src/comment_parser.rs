//! Comment parsing helpers for simple cloc-like line classification.
//!
//! Goals:
//! - Handle line comments (e.g. //, #, --) that may appear after code.
//! - Handle block comments that can start/end mid-line (e.g. /* ... */).
//! - Provide a best-effort treatment of string literals to avoid counting comment markers inside strings.
//!   This is intentionally lightweight; it won't be a full lexer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseState {
    pub in_block_comment: bool,
}

impl ParseState {
    pub fn new() -> Self {
        Self {
            in_block_comment: false,
        }
    }
}

/// Parse a line for languages with:
/// - line comment: `//`
/// - block comment: `/* */`
/// - string literals: single and double quotes
///
/// Returns whether the line contains code/comment, and updates state for multi-line block comments.
pub fn classify_line_c_like(line: &str, state: &mut ParseState) -> (bool, bool) {
    classify_line_generic(
        line,
        state,
        LineComment::DoubleSlash,
        Some(BlockComment::SlashStar),
        StringRules::CStyle,
    )
}

/// Python-like:
/// - line comment: `#`
/// - optional block comment: triple quotes (""" or ''')
///
/// Note: triple-quoted strings are not always comments in Python, but for cloc-style counting
/// treating them as comments is common.
pub fn classify_line_python_like(line: &str, state: &mut PythonState) -> (bool, bool) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return (false, false);
    }

    // If we are inside triple quote block, find a closing delimiter (best-effort: last occurrence).
    if let Some(delim) = state.in_triple {
        if let Some(end_idx) = trimmed.rfind(delim.as_str()) {
            state.in_triple = None;
            let tail = &trimmed[end_idx + delim.len()..];
            let (c2, com2, _hash2, _ended2) = scan_for_hash_comment_outside_strings(tail);
            return (c2, true || com2);
        }
        return (false, true);
    }

    // Outside triple quote block: scan for # (ignoring strings) and triple delimiters.
    let mut saw_code = false;
    let mut saw_comment = false;

    // First check for triple quote start outside strings.
    if let Some((idx, delim)) = find_triple_start_outside_strings(line) {
        // Before triple start, classify code/# comment.
        let before = &line[..idx];
        let (c, com, _hash, _ended) = scan_for_hash_comment_outside_strings(before);
        saw_code |= c;
        saw_comment |= com;

        // From triple start onwards, it becomes a comment block.
        // If it also ends on this line, we can still have code after.
        let after = &line[idx + delim.len()..];
        if let Some(end_idx) = find_substring_outside_strings(after, delim.as_str()) {
            saw_comment |= true;
            let tail = &after[end_idx + delim.len()..];
            let (c2, com2, _hash2, _ended2) = scan_for_hash_comment_outside_strings(tail);
            saw_code |= c2;
            saw_comment |= com2;
        } else {
            state.in_triple = Some(delim);
            saw_comment |= true;
        }

        return (saw_code, saw_comment);
    }

    // No triple quotes: just handle # comments.
    let (c, com, _hash, _ended) = scan_for_hash_comment_outside_strings(line);
    saw_code |= c;
    saw_comment |= com;

    (saw_code, saw_comment)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripleDelim {
    Double,
    Single,
}

impl TripleDelim {
    fn as_str(self) -> &'static str {
        match self {
            TripleDelim::Double => "\"\"\"",
            TripleDelim::Single => "'''",
        }
    }

    fn len(self) -> usize {
        3
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonState {
    pub in_triple: Option<TripleDelim>,
}

impl PythonState {
    pub fn new() -> Self {
        Self { in_triple: None }
    }
}

/// Lua-like:
/// - line comment: `--`
/// - block comment: `--[[ ]]` (basic form)
/// - string literals: single and double quotes
pub fn classify_line_lua_like(line: &str, state: &mut LuaState) -> (bool, bool) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return (false, false);
    }

    if state.in_long_comment {
        if trimmed.contains("]]" ) {
            // best-effort end detection; ignore strings here
            if let Some(pos) = trimmed.find("]]" ) {
                let after = &trimmed[pos + 2..];
                state.in_long_comment = false;
                if after.trim().is_empty() {
                    return (false, true);
                }
                // continue classifying remainder after end of long comment
                let (c2, com2) = classify_line_lua_like(after, state);
                return (c2, true || com2);
            }
        }
        return (false, true);
    }

    // Detect long comment start outside strings: --[[
    if let Some(idx) = find_substring_outside_strings(line, "--[[") {
        let before = &line[..idx];
        let (c_before, com_before) = classify_line_lua_line_comment(before);
        let after = &line[idx + 4..];

        // If it also ends on this line
        if let Some(end_idx) = find_substring_outside_strings(after, "]]" ) {
            let tail = &after[end_idx + 2..];
            let (c_tail, com_tail) = classify_line_lua_line_comment(tail);
            return (c_before || c_tail, true || com_before || com_tail);
        }

        state.in_long_comment = true;
        return (c_before, true || com_before);
    }

    classify_line_lua_line_comment(line)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuaState {
    pub in_long_comment: bool,
}

impl LuaState {
    pub fn new() -> Self {
        Self {
            in_long_comment: false,
        }
    }
}

/// XML/HTML-like: <!-- --> block comments. Strings are ignored.
pub fn classify_line_xml_like(line: &str, state: &mut ParseState) -> (bool, bool) {
    classify_line_generic(
        line,
        state,
        LineComment::None,
        Some(BlockComment::Xml),
        StringRules::None,
    )
}

/// CSS-like: /* */ block comments. Strings are ignored for now (CSS strings exist but uncommon in comment markers).
pub fn classify_line_css_like(line: &str, state: &mut ParseState) -> (bool, bool) {
    classify_line_generic(
        line,
        state,
        LineComment::None,
        Some(BlockComment::SlashStar),
        StringRules::None,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineComment {
    None,
    DoubleSlash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockComment {
    SlashStar,
    Xml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringRules {
    None,
    CStyle,
}

fn classify_line_generic(
    line: &str,
    state: &mut ParseState,
    line_comment: LineComment,
    block_comment: Option<BlockComment>,
    string_rules: StringRules,
) -> (bool, bool) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return (false, false);
    }

    let mut saw_code = false;
    let mut saw_comment = false;

    let mut i = 0usize;
    let bytes = line.as_bytes();

    let mut in_string_single = false;
    let mut in_string_double = false;

    while i < bytes.len() {
        // Handle block comment mode first
        if state.in_block_comment {
            // look for end delimiter
            if let Some((end_len, matched)) = match block_comment {
                Some(BlockComment::SlashStar) => match_at(bytes, i, b"*/").map(|_| (2, true)),
                Some(BlockComment::Xml) => match_at(bytes, i, b"-->").map(|_| (3, true)),
                None => None,
            } {
                if matched {
                    state.in_block_comment = false;
                    saw_comment = true;
                    i += end_len;
                    continue;
                }
            }
            // still in comment
            saw_comment = true;
            i += 1;
            continue;
        }

        // Handle strings (best-effort)
        if string_rules == StringRules::CStyle {
            let b = bytes[i];
            if in_string_single {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == b'\'' {
                    in_string_single = false;
                }
                i += 1;
                continue;
            }
            if in_string_double {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    in_string_double = false;
                }
                i += 1;
                continue;
            }

            if b == b'\'' {
                in_string_single = true;
                saw_code = true;
                i += 1;
                continue;
            }
            if b == b'"' {
                in_string_double = true;
                saw_code = true;
                i += 1;
                continue;
            }
        }

        // Block comment start
        if let Some(bc) = block_comment {
            let start = match bc {
                BlockComment::SlashStar => b"/*".as_slice(),
                BlockComment::Xml => b"<!--".as_slice(),
            };

            if match_at(bytes, i, start).is_some() {
                state.in_block_comment = true;
                saw_comment = true;
                i += start.len();
                continue;
            }
        }

        // Line comment start
        if line_comment == LineComment::DoubleSlash {
            if match_at(bytes, i, b"//").is_some() {
                // anything after is comment
                saw_comment = true;
                break;
            }
        }

        // Any non-whitespace outside comments is considered code.
        if !bytes[i].is_ascii_whitespace() {
            saw_code = true;
        }
        i += 1;
    }

    (saw_code, saw_comment)
}

fn match_at(hay: &[u8], idx: usize, needle: &[u8]) -> Option<()> {
    if idx + needle.len() > hay.len() {
        return None;
    }
    if &hay[idx..idx + needle.len()] == needle {
        Some(())
    } else {
        None
    }
}

fn scan_for_hash_comment_outside_strings(line: &str) -> (bool, bool, bool, bool) {
    // returns (saw_code, saw_comment, saw_hash_comment, ended_early)
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut saw_code = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_single = false;
            }
            saw_code = true;
            i += 1;
            continue;
        }
        if in_double {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_double = false;
            }
            saw_code = true;
            i += 1;
            continue;
        }

        if b == b'\'' {
            in_single = true;
            saw_code = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            saw_code = true;
            i += 1;
            continue;
        }

        if b == b'#' {
            return (saw_code, true, true, true);
        }

        if !b.is_ascii_whitespace() {
            saw_code = true;
        }
        i += 1;
    }

    (saw_code, false, false, false)
}

fn find_triple_start_outside_strings(line: &str) -> Option<(usize, TripleDelim)> {
    let mut i = 0usize;
    let bytes = line.as_bytes();
    let mut in_single = false;
    let mut in_double = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        if match_at(bytes, i, b"\"\"\"").is_some() {
            return Some((i, TripleDelim::Double));
        }
        if match_at(bytes, i, b"'''").is_some() {
            return Some((i, TripleDelim::Single));
        }

        if b == b'\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            i += 1;
            continue;
        }

        i += 1;
    }
    None
}

fn find_substring_outside_strings(haystack: &str, needle: &str) -> Option<usize> {
    // best-effort: for our uses in this file, any quoted string should be skipped.
    let bytes = haystack.as_bytes();
    let n = needle.as_bytes();
    let mut i = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    while i + n.len() <= bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        if b == b'\'' {
            in_single = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            i += 1;
            continue;
        }

        if &bytes[i..i + n.len()] == n {
            return Some(i);
        }

        i += 1;
    }

    None
}

fn classify_line_lua_line_comment(line: &str) -> (bool, bool) {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut saw_code = false;
    let mut saw_comment = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_single = false;
            }
            saw_code = true;
            i += 1;
            continue;
        }
        if in_double {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_double = false;
            }
            saw_code = true;
            i += 1;
            continue;
        }

        if match_at(bytes, i, b"--").is_some() {
            // everything after is comment
            saw_comment = true;
            break;
        }

        if b == b'\'' {
            in_single = true;
            saw_code = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_double = true;
            saw_code = true;
            i += 1;
            continue;
        }

        if !b.is_ascii_whitespace() {
            saw_code = true;
        }
        i += 1;
    }

    (saw_code, saw_comment)
}

