//! Whole-line comment classification for line budgets (§FS-001-config.3.1).
//! Best-effort: a line is a comment line only when it carries no code (a line
//! mixing code and a trailing comment is content); syntax is keyed off extension.

use std::path::Path;

use crate::LineStats;

/// Comment delimiters for one language family.
struct Syntax {
    line: &'static [&'static str],
    block: Option<(&'static str, &'static str)>,
}

const C_STYLE: Syntax = Syntax {
    line: &["//"],
    block: Some(("/*", "*/")),
};
const HASH: Syntax = Syntax {
    line: &["#"],
    block: None,
};
const DASH: Syntax = Syntax {
    line: &["--"],
    block: None,
};
const SEMI: Syntax = Syntax {
    line: &[";"],
    block: None,
};
const MARKUP: Syntax = Syntax {
    line: &[],
    block: Some(("<!--", "-->")),
};

fn syntax_for(path: &Path) -> Option<Syntax> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())?
        .to_ascii_lowercase();
    let syntax = match ext.as_str() {
        "rs" | "c" | "h" | "cpp" | "cc" | "hpp" | "hh" | "js" | "jsx" | "mjs" | "cjs" | "ts"
        | "tsx" | "go" | "java" | "kt" | "kts" | "swift" | "scala" | "cs" | "css" | "php"
        | "dart" | "zig" => C_STYLE,
        "py" | "rb" | "sh" | "bash" | "zsh" | "fish" | "toml" | "yaml" | "yml" | "ini" | "cfg"
        | "conf" | "pl" | "r" | "tf" | "dockerfile" => HASH,
        "lua" | "sql" | "hs" | "elm" | "adb" | "ads" => DASH,
        "clj" | "cljs" | "el" | "lisp" | "scm" | "rkt" => SEMI,
        "html" | "htm" | "xml" | "svg" | "vue" | "xhtml" => MARKUP,
        _ => return None,
    };
    Some(syntax)
}

/// Classify every physical line of `text` into total / blank / comment counts.
pub fn classify(path: &Path, text: &str) -> LineStats {
    let mut stats = LineStats::default();
    if text.is_empty() {
        return stats;
    }

    let syntax = syntax_for(path);
    let mut in_block = false;

    for line in text.lines() {
        stats.total += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            stats.blank += 1;
            continue;
        }
        let Some(syntax) = syntax.as_ref() else {
            continue;
        };
        let (is_comment, next_in_block) = classify_line(trimmed, in_block, syntax);
        in_block = next_in_block;
        if is_comment {
            stats.comment += 1;
        }
    }

    stats
}

/// Classify one non-blank, trimmed line. Returns `(is_comment_line, in_block_after)`.
fn classify_line(trimmed: &str, in_block: bool, syntax: &Syntax) -> (bool, bool) {
    if in_block {
        let (_, end) = syntax.block.expect("block state implies block syntax");
        return match trimmed.find(end) {
            // Block closes on this line; trailing code makes it a content line.
            Some(idx) => (trimmed[idx + end.len()..].trim().is_empty(), false),
            None => (true, true),
        };
    }

    if syntax.line.iter().any(|token| trimmed.starts_with(token)) {
        return (true, false);
    }

    if let Some((start, end)) = syntax.block
        && let Some(rest) = trimmed.strip_prefix(start)
    {
        return match rest.find(end) {
            // Opens and closes on one line; trailing code makes it a content line.
            Some(idx) => (rest[idx + end.len()..].trim().is_empty(), false),
            None => (true, true),
        };
    }

    (false, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(name: &str, text: &str) -> LineStats {
        classify(Path::new(name), text)
    }

    #[test]
    fn counts_blanks_and_line_comments() {
        let s = stats("a.rs", "fn a() {}\n\n// note\nfn b() {}\n");
        assert_eq!(s.total, 4);
        assert_eq!(s.blank, 1);
        assert_eq!(s.comment, 1);
    }

    #[test]
    fn block_comments_span_lines() {
        let s = stats("a.rs", "/* one\n   two */\ncode();\n");
        assert_eq!(s.total, 3);
        assert_eq!(s.comment, 2);
        assert_eq!(s.blank, 0);
    }

    #[test]
    fn trailing_code_after_block_close_is_content() {
        let s = stats("a.rs", "/* x */ let y = 1;\n");
        assert_eq!(s.comment, 0);
    }

    #[test]
    fn trailing_comment_after_code_is_content() {
        let s = stats("a.rs", "let y = 1; // note\n");
        assert_eq!(s.comment, 0);
    }

    #[test]
    fn hash_languages_use_pound() {
        let s = stats("x.toml", "# header\nkey = 1\n");
        assert_eq!(s.comment, 1);
    }

    #[test]
    fn unknown_and_prose_extensions_have_no_comments() {
        let s = stats("README.md", "# Title\n\ntext\n");
        assert_eq!(s.comment, 0);
        assert_eq!(s.blank, 1);
        assert_eq!(s.total, 3);
    }
}
