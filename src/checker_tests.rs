//! Unit tests for the checker core (§FS-004-check-audit). Kept in a sibling file
//! so `lib.rs` stays well under its own line budget.

use super::*;

#[test]
fn reports_hard_overflow_and_suppresses_soft_for_same_rule() {
    let checker = Checker::new(vec![Rule::new(
        "rust",
        Selector::Extension("rs".to_owned()),
        Budget::new(Unit::Lines, Some(2), Some(3)),
        MessageTemplate::new("split-rust", "Split {path}: {actual} {unit}."),
    )])
    .expect("valid checker");

    let file = measure_text("src/lib.rs", "a\nb\nc\n");
    let overflows = checker.check(&file).expect("check succeeds");

    assert_eq!(overflows.len(), 1);
    assert_eq!(overflows[0].severity, Severity::Hard);
    assert_eq!(overflows[0].actual, 3);
    assert_eq!(overflows[0].limit, 3);
    assert_eq!(
        overflows[0].finding_line(),
        "src/lib.rs: 3 lines > 3 lines [hard, rule: rust, message: split-rust]"
    );
}

#[test]
fn renders_message_template_variables() {
    let checker = Checker::new(vec![Rule::new(
        "domain",
        Selector::Prefix("src/domain/".to_owned()),
        Budget::new(Unit::Bytes, Some(5), None),
        MessageTemplate::new(
            "domain-split",
            "{severity} overflow in {path}; move code toward {rule} (§GOAL-008-remediation-messages).",
        ),
    )])
    .expect("valid checker");

    let file = measure_bytes("src/domain/order.rs", b"abcdef");
    let overflows = checker.check(&file).expect("check succeeds");
    let message = &overflows[0].message;

    assert_eq!(overflows[0].severity, Severity::Soft);
    assert_eq!(
        message.text,
        "soft overflow in src/domain/order.rs; move code toward domain (§GOAL-008-remediation-messages)."
    );
}

#[test]
fn validates_budget_order() {
    let error = Checker::new(vec![Rule::new(
        "bad",
        Selector::All,
        Budget::new(Unit::Lines, Some(10), Some(5)),
        MessageTemplate::new("bad-message", "Split it."),
    )])
    .expect_err("invalid checker");

    assert_eq!(
        error.to_string(),
        "invalid budget for rule bad: soft limit cannot be greater than hard limit"
    );
}

#[test]
fn token_rules_require_token_measurements() {
    let checker = Checker::new(vec![Rule::new(
        "tokens",
        Selector::All,
        Budget::new(Unit::Tokens, Some(100), None),
        MessageTemplate::new("token-split", "Reduce token load."),
    )])
    .expect("valid checker");

    let error = checker
        .check(&measure_text("README.md", "text"))
        .expect_err("tokens are missing");

    assert_eq!(
        error.to_string(),
        "missing tokens measurement for README.md under rule tokens"
    );
}

#[test]
fn selects_one_effective_rule_per_unit() {
    let checker = Checker::new(vec![
        Rule::new(
            "all-rust",
            Selector::Extension("rs".to_owned()),
            Budget::new(Unit::Lines, Some(5), Some(10)),
            MessageTemplate::new("all", "All rust."),
        ),
        Rule::new(
            "domain-rust",
            Selector::Prefix("src/domain/".to_owned()),
            Budget::new(Unit::Lines, Some(100), Some(200)),
            MessageTemplate::new("domain", "Domain rust."),
        ),
    ])
    .expect("valid checker");

    let file = measure_text("src/domain/order.rs", &"line\n".repeat(50));
    let overflows = checker.check(&file).expect("check succeeds");

    assert!(overflows.is_empty());
}

#[test]
fn keeps_different_units_together() {
    let checker = Checker::new(vec![
        Rule::new(
            "bytes",
            Selector::All,
            Budget::new(Unit::Bytes, Some(5), None),
            MessageTemplate::new("bytes", "Bytes."),
        ),
        Rule::new(
            "lines",
            Selector::All,
            Budget::new(Unit::Lines, Some(2), None),
            MessageTemplate::new("lines", "Lines."),
        ),
    ])
    .expect("valid checker");

    let file = measure_text("README.md", "one\ntwo\nthree\n");
    let overflows = checker.check(&file).expect("check succeeds");

    assert_eq!(overflows.len(), 2);
    assert!(
        overflows
            .iter()
            .any(|overflow| overflow.unit == Unit::Bytes)
    );
    assert!(
        overflows
            .iter()
            .any(|overflow| overflow.unit == Unit::Lines)
    );
}

#[test]
fn reports_ambiguous_same_unit_rules() {
    let checker = Checker::new(vec![
        Rule::new(
            "first",
            Selector::All,
            Budget::new(Unit::Bytes, Some(1), None),
            MessageTemplate::new("first", "First."),
        ),
        Rule::new(
            "second",
            Selector::All,
            Budget::new(Unit::Bytes, Some(2), None),
            MessageTemplate::new("second", "Second."),
        ),
    ])
    .expect("valid checker");

    let error = checker
        .check(&measure_bytes("README.md", b"abcdef"))
        .expect_err("rules are ambiguous");

    assert_eq!(
        error.to_string(),
        "ambiguous bytes rules for README.md: first, second"
    );
}

#[test]
fn priority_breaks_specificity_ties() {
    let checker = Checker::new(vec![
        Rule::new(
            "strict",
            Selector::All,
            Budget::new(Unit::Bytes, Some(1), None),
            MessageTemplate::new("strict", "Strict."),
        ),
        Rule::new(
            "relaxed",
            Selector::All,
            Budget::new(Unit::Bytes, Some(100), None),
            MessageTemplate::new("relaxed", "Relaxed."),
        )
        .with_priority(10),
    ])
    .expect("valid checker");

    let overflows = checker
        .check(&measure_bytes("README.md", b"abcdef"))
        .expect("check succeeds");

    assert!(overflows.is_empty());
}

#[test]
fn renders_template_values_without_replacing_inserted_placeholders() {
    let checker = Checker::new(vec![Rule::new(
        "rust",
        Selector::Exact("src/{limit}.rs".to_owned()),
        Budget::new(Unit::Lines, Some(1), None),
        MessageTemplate::new("split-rust", "Split {path}: limit {limit} {unit}."),
    )])
    .expect("valid checker");

    let file = measure_text("src/{limit}.rs", "one\ntwo\n");
    let overflows = checker.check(&file).expect("check succeeds");

    assert_eq!(
        overflows[0].message.text,
        "Split src/{limit}.rs: limit 1 lines."
    );
}

#[test]
fn line_policy_excludes_blanks_and_whole_line_comments() {
    // total physical lines = 6: 2 code, 2 comment, 2 blank.
    let text = "fn a() {}\n\n// comment one\n// comment two\n\nfn b() {}\n";

    let blanks_count = Rule::new(
        "raw",
        Selector::Extension("rs".to_owned()),
        Budget::new(Unit::Lines, Some(6), None),
        MessageTemplate::new("m", "{actual}"),
    )
    .with_line_policy(true, true);
    let checker = Checker::new(vec![blanks_count]).expect("valid");
    let overflows = checker
        .check(&measure_text("src/a.rs", text))
        .expect("check succeeds");
    assert_eq!(overflows[0].actual, 6, "all six physical lines counted");

    let code_only = Rule::new(
        "code-only",
        Selector::Extension("rs".to_owned()),
        Budget::new(Unit::Lines, Some(2), None),
        MessageTemplate::new("m", "{actual}"),
    )
    .with_line_policy(false, false);
    let checker = Checker::new(vec![code_only]).expect("valid");
    let overflows = checker
        .check(&measure_text("src/a.rs", text))
        .expect("check succeeds");
    assert_eq!(overflows[0].actual, 2, "only the two code lines counted");
}

#[test]
fn large_batch_smoke() {
    let checker = Checker::new(vec![Rule::new(
        "rust",
        Selector::Extension("rs".to_owned()),
        Budget::new(Unit::Lines, Some(200), Some(400)),
        MessageTemplate::new("split-rust", "Split {path}."),
    )])
    .expect("valid checker");

    let overflow_count: usize = (0..10_000)
        .map(|index| {
            let line_count = if index % 10 == 0 { 450 } else { 40 };
            let text = "fn helper() {}\n".repeat(line_count);
            let file = measure_text(format!("src/module_{index:05}.rs"), &text);
            checker.check(&file).expect("check succeeds").len()
        })
        .sum();

    assert_eq!(overflow_count, 1_000);
}
