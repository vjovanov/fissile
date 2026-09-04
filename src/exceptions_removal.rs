//! Repair-only registry loading for soft `exception remove`.

use super::*;

/// One soft-registry entry as `exception remove` may address it. The private
/// representation prevents an orphan from being mistaken for an [`Exception`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemovalEntry(Entry);

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
        Self(Entry::Resolved {
            entry,
            resolved_index,
        })
    }

    pub(crate) fn registry(&self) -> &str {
        match &self.0 {
            Entry::Resolved { entry, .. } => &entry.registry,
            Entry::OrphanShadow { registry, .. } => registry,
        }
    }

    pub(crate) fn path(&self) -> &str {
        match &self.0 {
            Entry::Resolved { entry, .. } => &entry.path,
            Entry::OrphanShadow { path, .. } => path,
        }
    }

    pub(crate) fn match_kind(&self) -> MatchKind {
        match &self.0 {
            Entry::Resolved { entry, .. } => entry.match_kind,
            Entry::OrphanShadow { match_kind, .. } => *match_kind,
        }
    }

    pub(crate) fn rules(&self) -> &[String] {
        match &self.0 {
            Entry::Resolved { entry, .. } => &entry.rules,
            Entry::OrphanShadow { rules, .. } => rules,
        }
    }

    pub(crate) fn max_value(&self) -> u64 {
        match &self.0 {
            Entry::Resolved { entry, .. } => entry.max_value,
            Entry::OrphanShadow { max_value, .. } => *max_value,
        }
    }

    pub(crate) fn max_unit(&self) -> Unit {
        match &self.0 {
            Entry::Resolved { entry, .. } => entry.max_unit,
            Entry::OrphanShadow { max_unit, .. } => *max_unit,
        }
    }

    pub(crate) fn resolved(&self) -> Option<(usize, &Exception)> {
        match &self.0 {
            Entry::Resolved {
                entry,
                resolved_index,
            } => Some((*resolved_index, entry)),
            Entry::OrphanShadow { .. } => None,
        }
    }

    /// Whether two repair views describe the same entry as written. A resolved
    /// index points into the loader's filtered [`Registries::soft`] vector, so
    /// deleting an earlier entry may renumber it without changing the document
    /// (§FS-009-exception-remove.5).
    pub(crate) fn same_written_entry(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (
                Entry::Resolved { entry, .. },
                Entry::Resolved {
                    entry: other_entry, ..
                },
            ) => entry == other_entry,
            (Entry::OrphanShadow { .. }, Entry::OrphanShadow { .. }) => self == other,
            _ => false,
        }
    }

    pub(crate) fn applies_to_rule(&self, rule: &str) -> bool {
        self.rules()
            .iter()
            .any(|listed| listed == "*" || listed == rule)
    }

    pub(crate) fn matches_path(&self, path: &str) -> bool {
        match &self.0 {
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
        for (document_index, raw) in soft_raw.into_iter().enumerate() {
            if orphaned.contains(&document_index) {
                soft_entries.push(build_orphan_shadow(raw, registry_path(soft))?);
            } else {
                let entry = build_exception(raw, Severity::Soft, registry_path(soft))?;
                let resolved_index = resolved.len();
                resolved.push(entry.clone());
                soft_entries.push(RemovalEntry::from_resolved(entry, resolved_index));
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
fn build_orphan_shadow(raw: RawException, registry: &str) -> Result<RemovalEntry, ExceptionError> {
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
    Ok(RemovalEntry(Entry::OrphanShadow {
        registry: registry.to_owned(),
        path: raw.path,
        match_kind: raw.match_kind,
        rules: raw.rules,
        max_value: raw.max_accepted.value,
        max_unit: raw.max_accepted.unit.into(),
        matcher,
    }))
}
