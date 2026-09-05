# E2E-093-audit-only-findings-is-the-gate-output: audit asked for findings gives the gate's answer

`--only findings` reduces `audit` to what `check` reports over the scan scope
(§FS-004-check-audit.2). This tree is clean and holds an exception entry, so the
default run prints `ok` and then the `exceptions:` counts; asked for `findings`
alone it prints `ok` and stops.

That pins where the success marker lives. It is what an empty findings section
prints (§FS-004-check-audit.1), not a line the run adds beside its sections — so
it appears when `findings` is named and stays away when it is not. A run asked
only for coverage that still printed `ok` would have made the marker a preamble,
which is exactly the byte §GOAL-004-token-thrift.1 says has to carry weight.
