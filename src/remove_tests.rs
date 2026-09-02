//! Unit tests for the registry rewrite `exception remove` performs
//! (§FS-009-exception-remove.4).

use super::*;

const REGISTRY: &str = "fissile_exceptions_version = 2\n\
    \n\
    [[exceptions]]\n\
    path = \"a.rs\"\n\
    max_accepted = { value = 400, unit = \"lines\" }\n\
    reason = \"\"\"\n\
    not this one: [[exceptions]]\n\
    \"\"\"\n\
    \n\
    [[exceptions]]\n\
    path = \"b.rs\"\n\
    max_accepted = { value = 500, unit = \"lines\" }\n";

fn remove(text: &str, index: usize) -> String {
    delete_block(text, index, Path::new("r"), "x").unwrap()
}

/// The block goes, and with it the blank line that separated it from the next
/// entry — so the entries that remain keep the spacing they had.
#[test]
fn removing_the_first_block_leaves_the_second_intact() {
    let out = remove(REGISTRY, 0);
    assert_eq!(
        out,
        "fissile_exceptions_version = 2\n\
         \n\
         [[exceptions]]\n\
         path = \"b.rs\"\n\
         max_accepted = { value = 500, unit = \"lines\" }\n"
    );
}

/// The last block takes the blank line that led into it, and the document keeps
/// exactly one trailing newline.
#[test]
fn removing_the_last_block_leaves_one_trailing_newline() {
    let out = remove(REGISTRY, 1);
    assert!(out.ends_with("\"\"\"\n"), "{out:?}");
    assert!(!out.contains("b.rs"));
    assert!(out.contains("not this one: [[exceptions]]"));
}

/// Removing the only entry leaves a registry that is empty rather than absent:
/// the version line stays, so the next `add` appends to a version-2 document.
#[test]
fn removing_the_only_block_keeps_the_version_line() {
    let registry = "fissile_exceptions_version = 2\n\
        \n\
        [[exceptions]]\n\
        path = \"a.rs\"\n";
    assert_eq!(remove(registry, 0), "fissile_exceptions_version = 2\n");
}

/// A `[[exceptions]]` line inside a `reason` is prose: counting it would shift
/// every index after it and cut the wrong entry out (§FS-008-exception-retune.3).
#[test]
fn a_reason_body_is_not_scanned() {
    let out = remove(REGISTRY, 1);
    assert!(out.contains("max_accepted = { value = 400, unit = \"lines\" }"));
    assert!(!out.contains("value = 500"));
}

/// A CRLF registry keeps its line endings: the `\r` belongs to the bytes of the
/// lines that stay.
#[test]
fn a_crlf_registry_keeps_its_line_endings() {
    let out = remove(&REGISTRY.replace('\n', "\r\n"), 0);
    assert!(out.contains("path = \"b.rs\"\r\n"), "{out:?}");
    assert!(
        out.split("\r\n").all(|line| !line.contains('\n')),
        "{out:?}"
    );
}

/// An index the document does not carry is refused rather than guessed at.
#[test]
fn an_absent_block_is_refused() {
    assert!(delete_block(REGISTRY, 2, Path::new("r"), "x").is_err());
}
