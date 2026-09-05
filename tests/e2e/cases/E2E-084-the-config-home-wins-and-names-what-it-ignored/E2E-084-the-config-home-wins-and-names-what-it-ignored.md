# E2E-084-the-config-home-wins-and-names-what-it-ignored: precedence is stated, not silent

With a config at both paths, `.agent-grounds/fissile.toml` takes effect and
`.agents/fissile.toml` is not read (§FS-001-config.8.3). The two fixtures carry
different rule ids and different thresholds, so the one printed line settles
which document was read; a merge of the two, or a fallback to the old one, would
each print something this assertion refuses.

The precedence alone is not enough. A reader who keeps editing
`.agents/fissile.toml` after a partial migration gets a passing run on every
change and none of their edits in force, and nothing in the output would tell
them why — the exact silent failure the move to a second directory could
otherwise introduce. So the run names the file it ignored and the one that won,
on stderr, and the exact stdout comparison keeps that line out of the findings.
