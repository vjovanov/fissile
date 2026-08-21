# E2E-038-an-unbuilt-file-is-not-a-dead-entry: an absent path is not a removed one

The registry accepts `src/moved.rs`, which is not in the working tree, and
`[exceptions].stale = "error"` would fail the run for a dead entry. The commit
removes nothing, so there is no dead entry and the run is `ok`
(§FS-004-check-audit.1.3).

A file can be missing for reasons that have nothing to do with the registry: a
build has not written it yet, or someone deleted it without staging the
deletion. Reading absence as removal would put every one of those between the
author and their next commit, over an entry that is still correct.
