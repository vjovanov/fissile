# E2E-082-init-writes-the-config-home: init installs the config at its new home

`fissile init` in a fresh repository writes `.agent-grounds/fissile.toml`
(§FS-002-init.2) and creates no `.agents/` directory at all. The second half is
the one that matters: a run that wrote both paths would leave every repository
it touched with two configs and no statement of which is in force.

`.agents/` is asserted as a directory rather than as `.agents/fissile.toml`.
Nothing in an `init` run has a reason to create it — the agent entrypoint is
`AGENTS.md` at the root (§FS-002-init.3) — so its absence is the stronger claim,
and the harness will not judge an `absent` path whose directory was never
created.

The `next:` block then has to name the file the run actually wrote
(§FS-002-init.5). A step pointing at `.agents/fissile.toml` after writing
somewhere else would be the first instruction a new user follows and a dead end.
