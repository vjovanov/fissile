//! Repair-only registry loading for soft `exception remove`.

use super::*;

/// One soft-registry entry as `exception remove` may address it. The private
/// representation prevents an orphan from being mistaken for an [`Exception`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemovalEntry {
    entry: Entry,
    written: WrittenEntry,
}

/// The durable meaning parsed from one `[[exceptions]]` block. This stays
/// separate from the resolved view: shadow resolution fills in inherited
/// rationale, while the write guard must remember that the document held a
/// pointer rather than that rationale (§FS-009-exception-remove.5).
#[derive(Clone, Debug, PartialEq, Eq)]
struct WrittenEntry {
    path: String,
    match_kind: MatchKind,
    rules: Vec<String>,
    max_value: u64,
    max_unit: Unit,
    until: Option<String>,
    reason: Option<String>,
    kind: Option<Kind>,
    shadows: Option<Shadows>,
    title: Option<String>,
    owner: Option<String>,
    issue: Option<String>,
}

impl WrittenEntry {
    fn from_raw(raw: &RawException) -> Self {
        Self {
            path: raw.path.clone(),
            match_kind: raw.match_kind,
            rules: raw.rules.clone(),
            max_value: raw.max_accepted.value,
            max_unit: raw.max_accepted.unit.into(),
            until: raw.until.clone(),
            reason: raw.reason.clone(),
            kind: raw.kind,
            shadows: raw.shadows,
            title: raw.title.clone(),
            owner: raw.owner.clone(),
            issue: raw.issue.clone(),
        }
    }

    /// Hard removal uses the ordinary resolved loader and never compares this
    /// snapshot. Keeping a complete value nevertheless makes every
    /// `RemovalEntry` describe one coherent entry rather than optional state.
    fn from_resolved(entry: &Exception) -> Self {
        Self {
            path: entry.path.clone(),
            match_kind: entry.match_kind,
            rules: entry.rules.clone(),
            max_value: entry.max_value,
            max_unit: entry.max_unit,
            until: Some(entry.until.clone()),
            reason: Some(entry.reason.clone()),
            kind: Some(entry.kind),
            shadows: None,
            title: entry.title.clone(),
            owner: entry.owner.clone(),
            issue: entry.issue.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Resolved {
        entry: Exception,
        resolved_index: usize,
    },
    OrphanShadow {
        registry: String,
        path: String,
        match_kind: MatchKind,
        rules: Vec<String>,
        max_value: u64,
        max_unit: Unit,
        matcher: Matcher,
    },
}

impl RemovalEntry {
    pub(crate) fn from_resolved(entry: Exception, resolved_index: usize) -> Self {
        let written = WrittenEntry::from_resolved(&entry);
        Self {
            entry: Entry::Resolved {
                entry,
                resolved_index,
            },
            written,
        }
    }

    fn from_written_resolved(
        entry: Exception,
        resolved_index: usize,
        written: WrittenEntry,
    ) -> Self {
        Self {
            entry: Entry::Resolved {
                entry,
                resolved_index,
            },
            written,
        }
    }

    pub(crate) fn registry(&self) -> &str {
        match &self.entry {
            Entry::Resolved { entry, .. } => &entry.registry,
            Entry::OrphanShadow { registry, .. } => registry,
        }
    }

    pub(crate) fn path(&self) -> &str {
        match &self.entry {
            Entry::Resolved { entry, .. } => &entry.path,
            Entry::OrphanShadow { path, .. } => path,
        }
    }

    pub(crate) fn match_kind(&self) -> MatchKind {
        match &self.entry {
            Entry::Resolved { entry, .. } => entry.match_kind,
            Entry::OrphanShadow { match_kind, .. } => *match_kind,
        }
    }

    pub(crate) fn rules(&self) -> &[String] {
        match &self.entry {
            Entry::Resolved { entry, .. } => &entry.rules,
            Entry::OrphanShadow { rules, .. } => rules,
        }
    }

    pub(crate) fn max_value(&self) -> u64 {
        match &self.entry {
            Entry::Resolved { entry, .. } => entry.max_value,
            Entry::OrphanShadow { max_value, .. } => *max_value,
        }
    }

    pub(crate) fn max_unit(&self) -> Unit {
        match &self.entry {
            Entry::Resolved { entry, .. } => entry.max_unit,
            Entry::OrphanShadow { max_unit, .. } => *max_unit,
        }
    }

    pub(crate) fn resolved(&self) -> Option<(usize, &Exception)> {
        match &self.entry {
            Entry::Resolved {
                entry,
                resolved_index,
            } => Some((*resolved_index, entry)),
            Entry::OrphanShadow { .. } => None,
        }
    }

    /// Whether two repair views describe the same entry as written. The
    /// canonical snapshot retains every durable field, including `shadows`,
    /// while excluding the resolved index and compiled matcher that exist only
    /// in loader state (§FS-009-exception-remove.5).
    pub(crate) fn same_written_entry(&self, other: &Self) -> bool {
        self.written == other.written
    }

    pub(crate) fn applies_to_rule(&self, rule: &str) -> bool {
        self.rules()
            .iter()
            .any(|listed| listed == "*" || listed == rule)
    }

    pub(crate) fn matches_path(&self, path: &str) -> bool {
        match &self.entry {
            Entry::Resolved { entry, .. } => entry.matches_path(path),
            Entry::OrphanShadow { matcher, .. } => match matcher {
                Matcher::Exact(expected) => expected == path,
                Matcher::Glob(glob) => glob.matches(path),
            },
        }
    }
}

impl Registries {
    /// Load the narrow repair view used only by soft `exception remove`.
    /// Missing hard twins remain addressable, while every other structural
    /// error follows the strict loader and the orphan never enters `Self`
    /// (§FS-009-exception-remove.2).
    pub(crate) fn load_for_soft_removal(
        soft: Option<RegistrySource<'_>>,
        hard: Option<RegistrySource<'_>>,
    ) -> Result<(Self, Vec<RemovalEntry>), ExceptionError> {
        let mut soft_raw = parse_raw(soft)?;
        // Capture the document before shadow resolution fills inherited fields.
        // The guard compares what was persisted, not the equivalent effective
        // `Exception` the loader derives from it.
        let written_entries: Vec<_> = soft_raw.iter().map(WrittenEntry::from_raw).collect();
        let hard_raw = parse_raw(hard)?;
        if let Some(entry) = hard_raw.iter().find(|entry| entry.shadows.is_some()) {
            return Err(ExceptionError::ShadowsInHardRegistry {
                site: site(registry_path(hard), &entry.path),
            });
        }
        let hard_entries = build_all(hard_raw, Severity::Hard, hard)?;
        let orphaned = resolve_shadows(
            &mut soft_raw,
            &hard_entries,
            registry_path(soft),
            hard,
            true,
        )?;

        let mut soft_entries = Vec::with_capacity(soft_raw.len());
        let mut resolved = Vec::with_capacity(soft_raw.len() - orphaned.len());
        for (document_index, (raw, written)) in
            soft_raw.into_iter().zip(written_entries).enumerate()
        {
            if orphaned.contains(&document_index) {
                soft_entries.push(build_orphan_shadow(raw, registry_path(soft), written)?);
            } else {
                let entry = build_exception(raw, Severity::Soft, registry_path(soft))?;
                let resolved_index = resolved.len();
                resolved.push(entry.clone());
                soft_entries.push(RemovalEntry::from_written_resolved(
                    entry,
                    resolved_index,
                    written,
                ));
            }
        }

        Ok((
            Self {
                soft: resolved,
                hard: hard_entries,
            },
            soft_entries,
        ))
    }
}

/// Validate the fields an orphan owns, without inventing the rationale that
/// only its absent hard twin could supply (§FS-009-exception-remove.2).
fn build_orphan_shadow(
    raw: RawException,
    registry: &str,
    written: WrittenEntry,
) -> Result<RemovalEntry, ExceptionError> {
    let site = || site(registry, &raw.path);
    if raw.max_accepted.value == 0 {
        return Err(ExceptionError::NonPositiveMax { site: site() });
    }
    if raw.rules.is_empty() {
        return Err(ExceptionError::NoRules { site: site() });
    }
    let matcher = match raw.match_kind {
        MatchKind::Exact => Matcher::Exact(raw.path.clone()),
        MatchKind::Glob => Matcher::Glob(Glob::new(raw.path.clone())),
    };
    Ok(RemovalEntry {
        entry: Entry::OrphanShadow {
            registry: registry.to_owned(),
            path: raw.path,
            match_kind: raw.match_kind,
            rules: raw.rules,
            max_value: raw.max_accepted.value,
            max_unit: raw.max_accepted.unit.into(),
            matcher,
        },
        written,
    })
}
