# E2E-027-measure-headroom-is-spendable: the headroom is room a caller can actually spend

`measure` exists so a caller can size an edit before writing it
(§FS-007-measure.2), which only works if the number means what it says: `n` more
units of the same kind leave every verdict where it stands.

A rule limit fires *at* the limit (§GOAL-006-graded-limits), so the last value
that clears an 8-line hard limit is 7 — and a 7-line file has zero room, not one.
The five-line file has two, and spending both leaves it at seven, still passing.
An agent told by the managed block to size new code by this number
(§FS-002-init.4) lands on the commit gate if the count is one too generous.
