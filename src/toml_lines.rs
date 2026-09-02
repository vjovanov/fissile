//! Reading a registry document's line structure as TOML rather than as text —
//! shared by `exception retune` and `exception remove`
//! (§FS-008-exception-retune.3, §FS-009-exception-remove.4). A `[[exceptions]]`
//! header or a `max_accepted` line written inside a `reason` is prose: it names
//! no entry, and counting it would shift every index after it.

/// Which multi-line string delimiter is currently open, if any. TOML closes a
/// `"""` string only with `"""` and a `'''` string only with `'''`, so the two
/// cannot be tracked as one toggle.
#[derive(Clone, Copy)]
enum Fence {
    Basic,
    Literal,
}

impl Fence {
    fn delimiter(self) -> &'static str {
        match self {
            Fence::Basic => "\"\"\"",
            Fence::Literal => "'''",
        }
    }
}

/// Whether each line *begins* outside every multi-line string — the lines that
/// carry TOML structure rather than the prose of a `reason`.
pub fn structural_lines(lines: &[String]) -> Vec<bool> {
    let mut open = None;
    lines
        .iter()
        .map(|line| {
            let structural = open.is_none();
            open = scan_line(line, open);
            structural
        })
        .collect()
}

/// The line each `[[exceptions]]` block begins on, in document order — the same
/// order the parsed entries carry, so an index into one indexes the other.
pub fn block_starts(lines: &[String]) -> Vec<usize> {
    let structural = structural_lines(lines);
    lines
        .iter()
        .enumerate()
        .filter(|(number, line)| structural[*number] && is_block_header(line))
        .map(|(number, _)| number)
        .collect()
}

fn is_block_header(line: &str) -> bool {
    line.trim().starts_with("[[exceptions]]")
}

/// Walk one line of TOML and report which multi-line string is open at its end.
/// Enough of a lexer to keep structure and prose apart: comments and single-line
/// strings are skipped so a `#` note or a `path = "a\"\"\"b"` value cannot open a
/// fence, and only a real delimiter changes the state.
fn scan_line(line: &str, mut open: Option<Fence>) -> Option<Fence> {
    let mut at = 0;
    while at < line.len() {
        if let Some(fence) = open {
            match line[at..].find(fence.delimiter()) {
                Some(offset) => {
                    at += offset + fence.delimiter().len();
                    open = None;
                }
                None => return Some(fence),
            }
            continue;
        }
        let rest = &line[at..];
        if rest.starts_with('#') {
            return None;
        }
        if rest.starts_with("\"\"\"") || rest.starts_with("'''") {
            let fence = if rest.starts_with('"') {
                Fence::Basic
            } else {
                Fence::Literal
            };
            open = Some(fence);
            at += fence.delimiter().len();
            continue;
        }
        if let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') {
            at += quote.len_utf8() + single_line_string(&rest[quote.len_utf8()..], quote);
            continue;
        }
        at += rest.chars().next().map_or(1, char::len_utf8);
    }
    open
}

/// The byte length consumed by a single-line string body and its closing quote.
/// An unterminated one runs to end of line, which is what a malformed registry
/// gets; the write is refused later by the caller's own check, never by a bad
/// guess here.
fn single_line_string(rest: &str, quote: char) -> usize {
    let escapes = quote == '"';
    let mut chars = rest.char_indices();
    while let Some((offset, character)) = chars.next() {
        if escapes && character == '\\' {
            chars.next();
            continue;
        }
        if character == quote {
            return offset + character.len_utf8();
        }
    }
    rest.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.split('\n').map(str::to_owned).collect()
    }

    /// A `[[exceptions]]`-shaped line inside a `reason` is prose in either
    /// multi-line form, and after a `#` it is a comment.
    #[test]
    fn a_header_in_prose_starts_no_block() {
        let text = "fissile_exceptions_version = 2\n\
            \n\
            [[exceptions]]\n\
            path = \"a.rs\"\n\
            reason = \"\"\"\n\
            [[exceptions]]\n\
            \"\"\"\n\
            \n\
            [[exceptions]]\n\
            path = \"b.rs\"\n\
            reason = '''\n\
            [[exceptions]]\n\
            '''\n\
            # [[exceptions]]\n";
        assert_eq!(block_starts(&lines(text)), vec![2, 8]);
    }

    /// A quoted fence inside a single-line string is a value, so the lines after
    /// it are still structure.
    #[test]
    fn a_quoted_fence_opens_no_string() {
        let text = "[[exceptions]]\n\
            title = \"a \\\"\\\"\\\" b\"\n\
            \n\
            [[exceptions]]\n";
        assert_eq!(block_starts(&lines(text)), vec![0, 3]);
    }
}
