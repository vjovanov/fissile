//! `fissile exception add` (§FS-005-exception-add): append a structured entry —
//! measure the file or take `--max`, quantize the ceiling, pick the soft or hard
//! registry, validate against §FS-003-exceptions, then append it.

use std::fs;
use std::path::PathBuf;

use crate::cli::{self, CommandError, Loaded};
use crate::entry::{self, Address};
use crate::exceptions::{INDEFINITE, Kind, MatchKind, is_indefinite};
use crate::{Rule, Severity, Unit, scan};

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
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub output: String,
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
    let unit = rules[0].budget.unit;
    let sizing = entry::Sizing {
        path: &path,
        match_kind: options.match_kind,
        max: options.max,
        unit: options.unit,
    };
    let base = entry::resolve_base(sizing, &loaded, unit, rules[0])?;
    entry::check_min_limit(&rules, options.severity, base)?;
    check_conflict(&loaded, options, &path, unit, rules[0])?;
    // The caller states a requirement; the step chooses the number written
    // (§FS-005-exception-add.2, §DF-006-quantized-ceilings.1).
    let max = entry::quantize(base, loaded.config.exceptions.bump.step(unit));

    let rendered = render_entry(options, &path, &until, unit, max);
    let registry_rel = entry::registry_path(&loaded, options.severity);
    let registry_path = loaded.root.join(&registry_rel);

    let existing = cli::read_optional(&registry_path)?;
    let base_text = existing.unwrap_or_else(|| "fissile_exceptions_version = 2\n".to_owned());
    let new_text = format!("{}\n{}\n", base_text.trim_end(), rendered);

    // Final guard: the combined registry must still validate (§FS-005-exception-add.4).
    entry::validate_combined(&loaded, options.severity, &new_text)?;

    if options.dry_run {
        return Ok(Run {
            output: format!("{rendered}\nwould update {}", registry_rel.display()),
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
    })
}

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
    rule: &Rule,
) -> Result<(), CommandError> {
    let address = Address {
        severity: options.severity,
        path,
        match_kind: options.match_kind,
        rules: &options.rules,
        unit,
    };
    let Some((_, existing)) = entry::locate(&loaded.registries, &address)? else {
        return Ok(());
    };
    // Named by where it lives, so the reader can go edit that entry instead of
    // adding a second one (§DF-005-exception-identity).
    let measured = match options.match_kind {
        MatchKind::Exact => entry::measure_value(loaded, path, unit, rule)
            .map(|value| format!("; the file is {value}"))
            .unwrap_or_default(),
        MatchKind::Glob => String::new(),
    };
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
