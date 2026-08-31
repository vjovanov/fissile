# E2E-014-init-names-entrypoint: the next block names the file that holds the block

`AGENTS.md` is the one entrypoint carrying real bytes; every other one `init`
touches is a link to it (§FS-002-init.3, §DF-009-one-file-agents-read). In a
repository shipping `CLAUDE.md` and no `AGENTS.md`, that file's content *is*
what the project already told agents, so it becomes `AGENTS.md` and `CLAUDE.md`
becomes a link — nothing the author wrote is lost, and the two paths can no
longer disagree.

The closing `see <path> ...` line must name the file that holds the block, not
the link (§FS-002-init.5, §GOAL-003-friendly-output). A reader following it is
looking for the workflow they just installed, and the link's target is where
editing it will stick.

That line is what this case asserts, so it holds on a host that cannot make
links at all: there the companion is `kept` and `AGENTS.md` still owns the
block. `E2E-042-companions-link-to-agents-md` asserts the link itself.
