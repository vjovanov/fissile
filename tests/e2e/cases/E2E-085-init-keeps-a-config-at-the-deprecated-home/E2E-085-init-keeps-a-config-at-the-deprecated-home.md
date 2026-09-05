# E2E-085-init-keeps-a-config-at-the-deprecated-home: an existing config is never replaced by a generated one

`fissile init` in a repository that already has `.agents/fissile.toml` and no
config at the new home writes no config at all (§FS-002-init.2). It reports the
existing file as `exists`, leaves it untouched, and says the path is deprecated
so the reader knows to move it (§FS-001-config.8.2).

The alternative reads as the helpful migration and is the worst outcome the move
could produce. A fully populated default written to `.agent-grounds/fissile.toml`
would win discovery on the very next run, so a project whose rules were tuned
over months would silently be governed by generic ones — with its own config
still on disk, still edited, and no longer enforced (§DF-012-config-home).

Two assertions carry that. `absent` says no second config appeared anywhere
under the new home. The `[[files]]` block says the old one is still the
project's: its own rule is present, and none of the generated default's rule ids
were written into it. The `next:` step then names the config this run found
rather than the one it would have written, because a step pointing at a file
that is not there is the first instruction a reader follows.
