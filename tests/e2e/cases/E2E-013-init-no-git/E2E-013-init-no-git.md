# E2E-013-init-no-git: init without a repository stays honest about the hook

Outside a git repository `init` still writes the config and agent instructions,
and the automatic hook install skips silently (§FS-002-init.6) — but the
`next:` block must not promise machinery that was not installed
(§FS-002-init.5): its hook step points at the repair (`git init && fissile
init`) or at wiring `fissile check --staged` into the project's own commit
flow.
