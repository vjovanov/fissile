//! `fissile exception add` (§FS-005-exception-add): append a structured entry —
//! measure the file or take `--max`, settle the ceiling, pick the soft or hard
//! registry, validate against §FS-003-exceptions, then append it.

use std::fs;
use std::path::PathBuf;

use crate::cli::{self, CommandError, Loaded};
use crate::entry::{self, Address};
use crate::exceptions::{INDEFINITE, Kind, MatchKind, is_indefinite};
use crate::{Severity, Unit, scan};

/// Inputs to `exception add`.
#[derive(Clone, Debug)]
pub struct AddOptions {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub path: String,
    pub severity: Severity,
    pub rules: Vec<String>,
    /// What the entry claims, and therefore what `reason` must establish
    /// (§FS-005-exception-add.1).
    pub kind: Kind,
    pub reason: String,
    /// Retirement condition. `None` is legal only for [`Kind::Structural`],
    /// which defaults it to `indefinite` (§FS-005-exception-add.1).
    pub until: Option<String>,
    pub match_kind: MatchKind,
    pub title: Option<String>,
    pub owner: Option<String>,
    pub issue: Option<String>,
    pub max: Option<u64>,
    pub unit: Option<Unit>,
    /// Whether the caller is a person at a terminal. The severity gate reads it
    /// (§DF-008-hard-severity-needs-a-terminal.1); the library takes it as a
    /// fact rather than probing, so a test can state either caller.
    pub interactive: bool,
    /// Proceed past the severity gate anyway (§FS-005-exception-add.4).
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub output: String,
    /// Said on stderr without changing the outcome (§FS-005-exception-add.4).
    pub warnings: Vec<String>,
}

pub fn run(options: &AddOptions) -> Result<Run, CommandError> {
    let loaded = cli::load(&options.root, options.config_path.as_deref())?;
    let path = match options.match_kind {
        MatchKind::Exact => scan::normalize_repo_path(&loaded.root, &options.path)?,
        MatchKind::Glob => options.path.replace('\\', "/"),
    };

    entry::validate_match(options.match_kind, &path)?;
    let until = resolve_until(options)?;
    let rules = entry::resolve_rules(&loaded, &options.rules)?;
    // After the flags are known well-formed, before anything is read about the
    // file: contradictory flags are named as such, and the refusal never
    // reports what the registry holds (§DF-008-hard-severity-needs-a-terminal.1).
    check_severity_gate(options)?;
    let unit = rules[0].budget.unit;
    let sizing = entry::Sizing {
        path: &path,
        match_kind: options.match_kind,
        max: options.max,
        unit: options.unit,
    };
    let base = entry::resolve_base(sizing, &loaded, unit, rules[0])?;
    entry::check_min_limit(
        &rules,
        options.severity,
        unit,
        &base,
        "no exception is needed here",
    )?;
    check_conflict(&loaded, options, &path, unit, base.measured)?;
    // A measurement is rounded to the step; a stated `--max` is the number
    // (§FS-005-exception-add.2, §DF-010-stated-ceilings-are-exact.1).
    let step = loaded.config.exceptions.bump.step(unit);
    let max = entry::ceiling(&base, step);
    // A soft ceiling on the hard limit is refused, and the refusal carries the
    // stated form that succeeds (§FS-005-exception-add.4).
    let binding = entry::binding_hard_limit(
        &rules,
        options.severity,
        has_hard_twin(&loaded, options, &path, unit),
    );
    entry::check_hard_limit(
        binding,
        &path,
        unit,
        &base,
        max,
        step,
        &entry::Routes {
            stated: route(options, options.severity, RouteMax::Placeholder(unit)),
            hard: route(options, Severity::Hard, RouteMax::AsGiven),
        },
    )?;

    let rendered = render_entry(options, &path, &until, unit, max);
    let registry_rel = entry::registry_path(&loaded, options.severity);
    let registry_path = loaded.root.join(&registry_rel);

    let existing = cli::read_optional(&registry_path)?;
    let base_text = existing.unwrap_or_else(|| "fissile_exceptions_version = 2\n".to_owned());
    let new_text = format!("{}\n{}\n", base_text.trim_end(), rendered);

    // Final guard: the combined registry must still validate (§FS-005-exception-add.4).
    entry::validate_combined(&loaded, options.severity, &new_text)?;

    let warnings = restatement_warning(options, &path, unit)
        .into_iter()
        .collect();

    if options.dry_run {
        return Ok(Run {
            output: format!("{rendered}\nwould update {}", registry_rel.display()),
            warnings,
        });
    }

    if let Some(parent) = registry_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&registry_path, &new_text)?;
    Ok(Run {
        output: format!(
            "appended {path} to {} (accepted up to {max} {unit})",
            registry_rel.display()
        ),
        warnings,
    })
}

/// A hard exception is the only way past a stop-the-line gate, so a person
/// decides it (§DF-008-hard-severity-needs-a-terminal.1). The refusal offers the
/// route an agent can take on its own, and names the flag a script needs.
fn check_severity_gate(options: &AddOptions) -> Result<(), CommandError> {
    if options.severity != Severity::Hard || options.force || options.interactive {
        return Ok(());
    }
    Err(CommandError::Usage(format!(
        "--severity hard records a human decision, and this is not a terminal. \
         A hard exception is the only way past a stop-the-line gate, so a person \
         reviews it. Record the debt at soft severity instead:\n  {}\n\
         Pass --force to add it anyway from a script.",
        soft_route(options),
    )))
}

/// Whether the hard registry holds this address. Above the hard limit a
/// deferred hard entry keeps the soft finding alive, which is what makes a soft
/// ceiling there legitimate (§DF-010-stated-ceilings-are-exact.2).
fn has_hard_twin(loaded: &Loaded, options: &AddOptions, path: &str, unit: Unit) -> bool {
    let address = Address {
        severity: Severity::Hard,
        path,
        match_kind: options.match_kind,
        rules: &options.rules,
        unit,
    };
    !entry::matching(&loaded.registries, &address).is_empty()
}

/// The offered command: this call with `--severity soft`, every other flag
/// carried through — it has to run as printed, and the `--kind` is the caller's
/// claim, not the gate's to substitute (§FS-005-exception-add.4).
fn soft_route(options: &AddOptions) -> String {
    route(options, Severity::Soft, RouteMax::AsGiven)
}

/// How a route spells `--max`: the caller's own, or a placeholder for the
/// number a hard-limit refusal asks them to state (§FS-005-exception-add.4).
enum RouteMax {
    AsGiven,
    Placeholder(Unit),
}

/// This call again at `severity`, every other flag carried through.
fn route(options: &AddOptions, severity: Severity, max: RouteMax) -> String {
    let mut command = format!("fissile exception add {}", shell_quote(&options.path));
    command.push_str(&format!(" --severity {severity}"));
    // Without the config the rerun loads a different one, and the rules the
    // caller named stop existing: a second refusal, which is the thing this
    // route is offered to avoid (§FS-005-exception-add.4).
    if let Some(config) = &options.config_path {
        command.push_str(&format!(
            " --config {}",
            shell_quote(&config.to_string_lossy())
        ));
    }
    for rule in &options.rules {
        command.push_str(&format!(" --rule {}", shell_quote(rule)));
    }
    if options.match_kind == MatchKind::Glob {
        command.push_str(" --match glob");
    }
    command.push_str(&format!(" --kind {}", options.kind));
    // A structural entry never expires and takes no `--until` (§FS-005-exception-add.1).
    if options.kind == Kind::Deferred {
        let until = options.until.as_deref().map(str::trim).unwrap_or("");
        command.push_str(&format!(
            " --until {}",
            // Quoted like any other value: `<what retires it>` bare is a
            // redirection, and a template the shell chokes on is not fillable.
            if until.is_empty() {
                shell_quote("<what retires it>")
            } else {
                shell_quote(until)
            }
        ));
    }
    command.push_str(&format!(" --reason {}", shell_quote(&options.reason)));
    match max {
        RouteMax::AsGiven => {
            if let (Some(max), Some(unit)) = (options.max, options.unit) {
                command.push_str(&format!(" --max {max} --unit {unit}"));
            }
        }
        RouteMax::Placeholder(unit) => command.push_str(&format!(" --max <N> --unit {unit}")),
    }
    // Metadata the entry would otherwise lose on the rerun.
    for (flag, value) in [
        ("--title", &options.title),
        ("--owner", &options.owner),
        ("--issue", &options.issue),
    ] {
        if let Some(value) = value {
            command.push_str(&format!(" {flag} {}", shell_quote(value)));
        }
    }
    command
}

/// A `sh`-safe rendering: bare when every character is safe unquoted,
/// single-quoted otherwise. A glob, a path with a space, and a reason with
/// punctuation all have to survive being copied out of the refusal, and quoting
/// only what needs it keeps the common line readable.
pub(crate) fn shell_quote(value: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "-_./:,+@".contains(c);
    if !value.is_empty() && value.chars().all(safe) {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// A reason that says nothing the finding did not (§FS-005-exception-add.4).
/// Strip the entry's own facts and count what is left: this catches a reason
/// that is *entirely* restatement, which is why it warns rather than refuses.
fn restatement_warning(options: &AddOptions, path: &str, unit: Unit) -> Option<String> {
    let mut remaining = options.reason.to_lowercase();
    // Paths and rule ids are multi-token identifiers, so they come out as
    // substrings; the unit is one word and comes out as one, or `pipeline`
    // would be scored as `pipe` (§FS-005-exception-add.4).
    for fact in [path, &options.path] {
        remaining = remaining.replace(&fact.to_lowercase(), " ");
    }
    for rule in &options.rules {
        remaining = remaining.replace(&rule.to_lowercase(), " ");
    }
    let plural = unit.to_string();
    let singular = plural.strip_suffix('s').unwrap_or(&plural);
    let words = remaining
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .filter(|word| *word != plural && *word != singular)
        .count();
    if words >= RESTATEMENT_WORDS {
        return None;
    }
    Some(
        "--reason may only restate the finding. A reason names the constraint that makes \
         splitting illegal (--kind structural), or the boundary that is missing and what \
         must exist first (--kind deferred). What the file contains is what the finding \
         already said."
            .to_owned(),
    )
}

/// Words left after the facts are removed, below which a reason reads as a
/// restatement (§FS-005-exception-add.4).
const RESTATEMENT_WORDS: usize = 5;

/// Reconcile `--until` with `--kind` (§FS-005-exception-add.1): a structural
/// entry never expires, a deferred one must name what retires it. Each error
/// offers the other kind, usually the real correction.
fn resolve_until(options: &AddOptions) -> Result<String, CommandError> {
    let until = options.until.as_deref().map(str::trim);
    match (options.kind, until) {
        (Kind::Structural, None) => Ok(INDEFINITE.to_owned()),
        (Kind::Structural, Some(until)) if is_indefinite(until) => Ok(until.to_owned()),
        (Kind::Structural, Some(_)) => Err(CommandError::Usage(format!(
            "--kind structural never expires, so --until must be \"{INDEFINITE}\" or omitted; \
             if the work you named would retire it, this is --kind deferred"
        ))),
        (Kind::Deferred, None) | (Kind::Deferred, Some("")) => Err(CommandError::Usage(
            "--kind deferred requires --until naming the condition that retires the entry; \
             use --kind structural if an architectural constraint makes the split illegal"
                .to_owned(),
        )),
        (Kind::Deferred, Some(until)) if is_indefinite(until) => Err(CommandError::Usage(format!(
            "--kind deferred cannot be --until \"{INDEFINITE}\": name what retires the entry, \
             or use --kind structural if splitting the file is genuinely illegal"
        ))),
        (Kind::Deferred, Some(until)) => Ok(until.to_owned()),
    }
}

/// Reject a second entry answering an address the registry already answers
/// (§FS-005-exception-add.4). "An entry exists" is not "the file is accepted", so
/// the refusal carries the ceiling, the measurement, and the command that moves it.
fn check_conflict(
    loaded: &Loaded,
    options: &AddOptions,
    path: &str,
    unit: Unit,
    measured: Option<u64>,
) -> Result<(), CommandError> {
    let address = Address {
        severity: options.severity,
        path,
        match_kind: options.match_kind,
        rules: &options.rules,
        unit,
    };
    // Any overlapping entry is the conflict, so the first is enough to name; two
    // exact entries a glob spans are a legal registry, not a second fault
    // (§FS-003-exceptions.4).
    let Some((_, existing)) = entry::matching(&loaded.registries, &address)
        .into_iter()
        .next()
    else {
        return Ok(());
    };
    // Named by where it lives, so the reader edits that entry instead of adding
    // a second one (§DF-005-exception-identity). The measurement is the one
    // `resolve_base` took: a refusal is not worth a second run of the counter.
    let measured = measured.map_or_else(String::new, |value| format!("; the file is {value}"));
    // An entry reached through a glob is not the path the caller named, so the
    // message keeps both: the address to edit, and what it covers.
    let covering = if existing.path == path {
        String::new()
    } else {
        format!(" covering {path}")
    };
    Err(CommandError::Usage(format!(
        "{}: {} already has an entry{covering} for this rule and unit \
         (accepts up to {} {unit}{measured}) — move the ceiling with `fissile exception retune`",
        existing.registry, existing.path, existing.max_value
    )))
}

fn render_entry(options: &AddOptions, path: &str, until: &str, unit: Unit, max: u64) -> String {
    // No id line: the entry is identified by this registry, this path, and what
    // it accepts (§FS-005-exception-add.3, §DF-005-exception-identity).
    let mut lines = vec!["[[exceptions]]".to_owned()];
    if let Some(title) = &options.title {
        lines.push(format!("title = {}", entry::quote(title)));
    }
    lines.push(format!("path = {}", entry::quote(path)));
    lines.push(format!(
        "match = {}",
        entry::quote(match_str(options.match_kind))
    ));
    lines.push(format!("rules = [{}]", rule_list(&options.rules)));
    // `kind` and `until` are always written, even when `until` took the
    // structural default, so the entry never depends on a reader knowing the
    // command's defaults (§FS-005-exception-add.3).
    lines.push(format!(
        "kind = {}",
        entry::quote(&options.kind.to_string())
    ));
    lines.push(entry::max_accepted_line(max, unit));
    lines.push(format!("until = {}", entry::quote(until)));
    if let Some(owner) = &options.owner {
        lines.push(format!("owner = {}", entry::quote(owner)));
    }
    if let Some(issue) = &options.issue {
        lines.push(format!("issue = {}", entry::quote(issue)));
    }
    lines.push(format!(
        "reason = \"\"\"\n{}\n\"\"\"",
        options.reason.trim()
    ));
    lines.join("\n")
}

fn rule_list(rules: &[String]) -> String {
    rules
        .iter()
        .map(|rule| entry::quote(rule))
        .collect::<Vec<_>>()
        .join(", ")
}

fn match_str(match_kind: MatchKind) -> &'static str {
    match match_kind {
        MatchKind::Exact => "exact",
        MatchKind::Glob => "glob",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(reason: &str) -> AddOptions {
        AddOptions {
            root: PathBuf::from("."),
            config_path: None,
            path: "src/big.rs".to_owned(),
            severity: Severity::Soft,
            rules: vec!["rust-source".to_owned()],
            kind: Kind::Structural,
            reason: reason.to_owned(),
            until: None,
            match_kind: MatchKind::Exact,
            title: None,
            owner: None,
            issue: None,
            max: None,
            unit: None,
            interactive: false,
            force: false,
            dry_run: false,
        }
    }

    /// §FS-005-exception-add.4: a reason built from the finding's own facts
    /// leaves nothing behind once those facts are removed.
    #[test]
    fn a_reason_made_of_the_findings_facts_warns() {
        let options = options("src/big.rs is 612 lines, over the 550-line limit");
        assert!(restatement_warning(&options, "src/big.rs", Unit::Lines).is_some());
    }

    /// A claim a reviewer can disagree with survives the same subtraction.
    #[test]
    fn a_reason_that_names_a_constraint_does_not_warn() {
        let options = options(
            "The generator owns this whole file and asserts it byte-identical; \
             a split loses the incident-to-case mapping.",
        );
        assert!(restatement_warning(&options, "src/big.rs", Unit::Lines).is_none());
    }

    /// The unit name is one word, not a substring: a reason whose distinguishing
    /// words merely contain it keeps them (§FS-005-exception-add.4).
    #[test]
    fn the_unit_name_is_subtracted_as_a_word() {
        let options = options("The inline pipeline baseline is asserted here");
        assert!(restatement_warning(&options, "src/big.rs", Unit::Lines).is_none());
    }

    /// The gate reads the caller, not the severity alone: soft is the agent's
    /// to record (§DF-008-hard-severity-needs-a-terminal.1).
    #[test]
    fn the_severity_gate_stops_only_a_scripted_hard_add() {
        let mut scripted = options("a claim about a missing boundary here");
        scripted.severity = Severity::Hard;
        assert!(check_severity_gate(&scripted).is_err());

        assert!(check_severity_gate(&options("a claim")).is_ok());

        let mut forced = scripted.clone();
        forced.force = true;
        assert!(check_severity_gate(&forced).is_ok());

        let mut human = scripted.clone();
        human.interactive = true;
        assert!(check_severity_gate(&human).is_ok());
    }

    /// The refusal's offered command must run as printed, and must not restate
    /// the caller's claim as a different one (§FS-005-exception-add.4).
    #[test]
    fn the_offered_route_carries_the_calls_own_flags() {
        let mut scripted = options("the generator owns this file byte-identically");
        scripted.severity = Severity::Hard;

        let structural = soft_route(&scripted);
        assert_eq!(
            structural,
            "fissile exception add src/big.rs --severity soft --rule rust-source \
             --kind structural --reason 'the generator owns this file byte-identically'"
        );

        let mut deferred = scripted.clone();
        deferred.kind = Kind::Deferred;
        deferred.until = Some("the parser moves to its own module".to_owned());
        assert!(
            soft_route(&deferred)
                .contains("--kind deferred --until 'the parser moves to its own module'")
        );

        // No `--until` to carry: the placeholder stands in, and the flag stays.
        // It is quoted, or `<what` is a redirection and the line will not parse.
        let mut open = deferred.clone();
        open.until = None;
        assert!(soft_route(&open).contains("--kind deferred --until '<what retires it>'"));
    }

    /// Every flag that changes what the rerun loads or writes is carried, or the
    /// offered command produces a different entry — or a second refusal
    /// (§FS-005-exception-add.4).
    #[test]
    fn the_offered_route_carries_the_config_and_the_metadata() {
        let mut scripted = options("the generator owns this file byte-identically");
        scripted.severity = Severity::Hard;
        scripted.config_path = Some(PathBuf::from("build/fissile.toml"));
        scripted.title = Some("generated orders".to_owned());
        scripted.owner = Some("payments".to_owned());
        scripted.issue = Some("#12".to_owned());

        let command = soft_route(&scripted);
        assert!(command.contains(" --config build/fissile.toml"));
        assert!(command.contains(" --title 'generated orders'"));
        assert!(command.contains(" --owner payments"));
        assert!(command.contains(" --issue '#12'"));
    }

    /// A value the shell would rewrite is quoted; one it would not is left bare
    /// (§FS-005-exception-add.4).
    #[test]
    fn a_glob_or_a_spaced_path_survives_the_shell() {
        let mut glob = options("the generated modules are asserted byte-identical");
        glob.severity = Severity::Hard;
        glob.path = "src/*.rs".to_owned();
        glob.match_kind = MatchKind::Glob;
        assert!(soft_route(&glob).starts_with("fissile exception add 'src/*.rs' --severity soft"));

        let mut spaced = glob.clone();
        spaced.path = "src/odd dir/big.rs".to_owned();
        spaced.match_kind = MatchKind::Exact;
        assert!(soft_route(&spaced).contains("add 'src/odd dir/big.rs' "));

        // A plain path is not worth quoting, and E2E-034 asserts it bare.
        assert!(soft_route(&glob).contains(" --rule rust-source "));
    }
}
