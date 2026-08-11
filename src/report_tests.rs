//! Unit tests for evaluation against the registries (§FS-003-exceptions.3) and
//! for grouped text rendering (§FS-004-check-audit.1). Kept in a sibling file so
//! `report.rs` stays well under its own line budget.

use super::*;
use crate::config::Config;
use crate::{RenderedMessage, Unit};

const CONFIG: &str = r#"
fissile_config_version = 1
[[messages]]
id = "m"
text = "Split it."
[[rules]]
id = "rust"
include = ["src/**/*.rs"]
unit = "lines"
soft = 100
hard = 200
message = "m"
"#;

/// A hard registry accepting `src/big.rs` up to 250 lines, with `kind` written
/// verbatim so a pre-`kind` entry can be spelled as the empty string.
fn hard_registry(kind: &str, until: &str) -> String {
    format!(
        r#"
fissile_exceptions_version = 1
[[exceptions]]
id = "EX-001-big"
path = "src/big.rs"
match = "exact"
rules = ["rust"]
{kind}
max_accepted = {{ value = 250, unit = "lines" }}
until = "{until}"
reason = "a reason"
"#
    )
}

/// Every outcome `evaluate_file` produced for a `lines`-long `src/big.rs`, as
/// `(severity, is_reported)` in emission order.
fn outcomes(registry: &str, lines: u64) -> Vec<(Severity, bool)> {
    let checker = Config::parse(CONFIG).unwrap().to_checker().unwrap();
    let registries = Registries::load(None, Some(registry)).unwrap();
    let file = FileMeasurement::new("src/big.rs", lines * 12).with_lines(lines);
    evaluate_file(&checker, &registries, &file)
        .unwrap()
        .iter()
        .map(|outcome| (outcome.overflow().severity, outcome.is_reported()))
        .collect()
}

/// §FS-003-exceptions.3: a `deferred` hard entry accepts the blocking finding and
/// leaves the soft one standing. An entry declaring no kind reads as deferred and
/// behaves the same (§FS-003-exceptions.2.1).
#[test]
fn a_deferred_hard_exception_leaves_the_soft_finding_standing() {
    let standing = vec![(Severity::Hard, false), (Severity::Soft, true)];
    let until = "the case-builder module lands";
    assert_eq!(
        outcomes(&hard_registry("kind = \"deferred\"", until), 250),
        standing
    );
    assert_eq!(outcomes(&hard_registry("", until), 250), standing);
}

/// §FS-003-exceptions.3: a `structural` hard entry silences the soft finding too.
/// Splitting is illegal, so the warning can never be cleared by doing the work.
#[test]
fn a_structural_hard_exception_silences_the_soft_finding_too() {
    assert_eq!(
        outcomes(&hard_registry("kind = \"structural\"", "indefinite"), 250),
        vec![(Severity::Hard, false)]
    );
}

/// §FS-003-exceptions.3: the rule reaches only as far as a hard finding does. A
/// file below the hard limit never consults the hard registry, so the soft
/// warning stands and the soft registry is what accepts it.
#[test]
fn a_file_below_the_hard_limit_warns_despite_a_structural_hard_entry() {
    let registry = hard_registry("kind = \"structural\"", "indefinite");
    assert_eq!(outcomes(&registry, 150), vec![(Severity::Soft, true)]);
}

/// §GOAL-006-graded-limits.1: a *standing* hard finding suppresses the soft one
/// whatever the registries say, so a file grown past its ceiling reports the hard
/// overflow alone — the kind never reaches that path.
#[test]
fn a_hard_overflow_past_its_ceiling_still_suppresses_the_soft_finding() {
    for (kind, until) in [
        ("kind = \"structural\"", "indefinite"),
        ("kind = \"deferred\"", "the case-builder module lands"),
    ] {
        assert_eq!(
            outcomes(&hard_registry(kind, until), 300),
            vec![(Severity::Hard, true)]
        );
    }
}

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

/// §FS-003-exceptions.5: the attribution line names the file, the severity that
/// says which registry accepted it, and the ceiling. There is no entry id to
/// quote — an entry has none (§DF-005-exception-identity).
#[test]
fn a_silenced_overflow_is_attributed_by_path_and_ceiling() {
    let outcome = reported(
        "src/orders.rs",
        "source",
        Severity::Hard,
        620,
        "Must split.",
    );

    assert_eq!(
        silenced_line(outcome.overflow(), 620),
        "src/orders.rs: hard exception (accepted up to 620 lines)"
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
