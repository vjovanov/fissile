# DF-011-rule-local-exclusions: A rule opts paths out with its own exclusions, not with a thresholdless winner

The same file can carry several independent costs. An append-only changelog is
not meaningfully split by line count, but it should still be protected against a
large binary or generated blob by the byte catch-all. `[scan].exclude` cannot
express that distinction because it removes the path before every rule sees it,
and an exception would claim that an overflow was accepted rather than that the
line budget never applied (§FS-001-config.3.3).

The issue proposed two ways to express the missing scope: negative globs on the
rule that does not apply, or a thresholdless rule made specific enough to win
the `(file, unit)` overlap and mean "do not measure this unit."

## 1. Decision

A rule has an optional `exclude` glob list. Its scope is its positive `include`
list minus that negative list (§FS-001-config.3.4). Exclusion is settled before
priority and specificity choose among applicable rules, and it affects no other
rule or measurement unit.

Rules remain budgets: each one must declare `soft`, `hard`, or both. There is no
thresholdless opt-out rule.

## 2. Why

Negative scope states the local fact directly: this rule does not apply to this
path. A thresholdless winner states it indirectly through the overlap algorithm,
so changing an unrelated rule's priority or specificity could silently restore
or remove a budget. It would also put a rule with no budget into inventory and
coverage surfaces built to answer what the repository enforces.

Keeping exclusion on the rule gives checking, measurement, audit, and `limits`
one inspectable selector to share. It also preserves the useful validation that
every rule imposes a threshold; a misspelled or unfinished rule cannot silently
become an opt-out.

## 3. Consequences

- `[scan].exclude` remains the repository-wide way to remove a path from all
  measurement; `rules[].exclude` removes it from one rule only.
- An excluded rule never enters same-unit priority or specificity resolution,
  so it cannot win or create an ambiguity for that path.
- Rule inventory exposes a non-empty exclusion list, while omission and an empty
  list preserve the old text and JSON shapes (§FS-010-limits.3,
  §FS-010-limits.4).
- Exceptions remain records of accepted overflows and do not substitute for
  scope.

## 4. Rejected Alternative

**Allow a thresholdless rule to win a unit and suppress measurement.** This
overloads precedence with negation, makes the opt-out depend on other selectors,
and weakens the invariant that a rule describes a real budget. It also cannot
plainly express that only one existing rule is excluded while another rule for
the same unit should remain eligible.
