# E2E-090-audit-only-top-still-needs-a-count: the one section naming does not compute

Naming a section is normally the request to compute it: `--only coverage` needs
no `--rule-coverage`, and `--only stale` needs no `--stale-exceptions`
(§FS-004-check-audit.2). `top` is the exception, and this case is what keeps it
from being tidied away as an inconsistency.

Every other section is a question with one answer. `top` is a question with a
parameter, and no count is defensible as a default — one repository's useful
ranking is another's whole inventory. Naming the section says which ranking to
print, not how far down it goes, so the rule cannot reach it and `--top <N>`
carries the number it needs.

The refusal names that flag, so the caller reads what to add rather than that
something was missing.
