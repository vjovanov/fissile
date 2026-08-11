//! Unit tests for grouped text rendering (§FS-004-check-audit.1). Kept in a
//! sibling file so `report.rs` stays well under its own line budget.

use super::*;
use crate::{RenderedMessage, Unit};

fn reported(path: &str, rule: &str, severity: Severity, actual: u64, text: &str) -> Outcome {
    Outcome::Reported(Overflow {
        path: path.into(),
        rule_id: rule.to_owned(),
        severity,
        unit: Unit::Lines,
        actual,
        limit: match severity {
            Severity::Soft => 350,
            Severity::Hard => 550,
        },
        message: RenderedMessage {
            id: format!("{severity}-guidance"),
            text: text.to_owned(),
        },
    })
}

#[test]
fn files_sharing_guidance_are_listed_under_one_copy_of_it() {
    let outcomes = [
        reported(
            "src/util.rs",
            "source",
            Severity::Soft,
            410,
            "Should split.",
        ),
        reported("src/big.rs", "source", Severity::Hard, 620, "Must split."),
        reported("src/tax.rs", "source", Severity::Soft, 502, "Should split."),
    ];

    let blocks = finding_blocks(&outcomes, false);

    // Hard first, and the soft guidance is written once for both its files.
    assert_eq!(
        blocks,
        vec![
            "hard: 1 file over the 550-line budget [rule: source, message: hard-guidance]\n  \
             Must split.\n    src/big.rs: 620 lines"
                .to_owned(),
            "soft: 2 files over the 350-line budget [rule: source, message: soft-guidance]\n  \
             Should split.\n    src/tax.rs: 502 lines\n    src/util.rs: 410 lines"
                .to_owned(),
        ]
    );
}

/// §FS-001-config.4: guidance that names a file cannot stand for another file,
/// so a `{path}` template still renders one block per file.
#[test]
fn per_file_guidance_does_not_collapse() {
    let outcomes = [
        reported("src/a.rs", "source", Severity::Soft, 400, "Split src/a.rs."),
        reported("src/b.rs", "source", Severity::Soft, 380, "Split src/b.rs."),
    ];

    let blocks = finding_blocks(&outcomes, false);

    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].ends_with("Split src/a.rs.\n    src/a.rs: 400 lines"));
    assert!(blocks[1].ends_with("Split src/b.rs.\n    src/b.rs: 380 lines"));
}

#[test]
fn guidance_wraps_at_a_fixed_width() {
    let long = "Move a cohesive group of items into a sibling module along a seam \
                that already exists, rather than cutting the file at the line count.";
    let outcomes = [reported("src/a.rs", "source", Severity::Soft, 400, long)];

    let block = finding_blocks(&outcomes, false).remove(0);
    let guidance: Vec<&str> = block
        .lines()
        .filter(|line| line.starts_with("  ") && !line.starts_with("    "))
        .collect();

    assert!(guidance.len() > 1, "the long guidance wrapped");
    for line in &guidance {
        assert!(line.chars().count() <= GUIDANCE_COLUMNS + 2, "{line}");
    }
    // Wrapping only inserts line breaks; no word is lost or broken.
    let rejoined: Vec<&str> = guidance
        .iter()
        .flat_map(|line| line.split_whitespace())
        .collect();
    assert_eq!(rejoined, long.split_whitespace().collect::<Vec<_>>());
}

/// Explicit newlines are the project's own paragraphing and survive wrapping.
#[test]
fn newlines_in_a_message_are_kept() {
    let outcomes = [reported(
        "src/a.rs",
        "source",
        Severity::Soft,
        400,
        "Should split.\nfissile exception add <path> --severity soft --rule source",
    )];

    let block = finding_blocks(&outcomes, false).remove(0);

    assert!(block.contains(
        "\n  Should split.\n  fissile exception add <path> --severity soft --rule source\n"
    ));
}
