# E2E-031-exception-add-measures-once: a refusal reuses the measurement it already took

`exception add` measures the file to size the ceiling, then may refuse because an
entry already answers the address — and the refusal quotes the measurement so the
caller can weigh the ceiling against the file (§FS-005-exception-add.4).

Quoting it must not cost a second measurement. Under a token unit the count comes
from an external command (§DA-001-token-external-command), so measuring twice
runs a tokenizer subprocess twice for one refusal, against
§GOAL-001-fast-feedback. The counter here keeps a ledger, and the scenario reads
it: one invocation for one run. Unix-gated for the same reason as
§E2E-009-check-token-rule — the stub is a POSIX shell script, not the mode it
stands in for.
