# E2E-086-init-force-keeps-a-config-at-the-deprecated-home: `--force` does not reach the config either

`fissile init --force` in a repository whose only config is at
`.agents/fissile.toml` still writes no config (§FS-002-init.2). It reports the
existing file as `exists`, leaves it untouched, and says the path is deprecated
(§FS-001-config.8.2) — the same answer a bare `init` gives in E2E-085.

The flag is what makes this worth its own scenario. `--force` exists to
re-generate the documents `init` owns, and a reader who knows that has every
reason to expect it to reach the config too. It must not: writing the generated
default to `.agent-grounds/fissile.toml` would win discovery on the next run and
silently govern a project whose own rules are still on disk
(§DF-012-config-home). That is the single worst outcome the move to the new home
could produce, and `--force` is the shortest path to it.

The assertions are E2E-085's, because the promise is that the flag changes
nothing: `absent` says no second config appeared under the new home, and the
`[[files]]` block says the old one is still the project's — its own rule
present, none of the generated default's rule ids written into it.
