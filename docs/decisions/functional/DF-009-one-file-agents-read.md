# DF-009-one-file-agents-read: `AGENTS.md` holds the block; every other entrypoint is a link to it.

`init` wrote the same managed block into every agent entrypoint it touched. A
repository with `AGENTS.md` and `CLAUDE.md` carried two copies; one with a
`.claude/` directory carried a third, because §FS-002-init.3 treats that
directory as reason to write `.claude/CLAUDE.md`. Claude Code reads both the
root file and the `.claude/` one, so the same five lines arrived twice in the
same context window — from the tool whose whole purpose is spending fewer
tokens on files agents read (§GOAL-004-token-thrift).

Duplication also drifts. Each copy is upgraded independently, so a run that
fails partway, or a file someone edited by hand, leaves two generations of
instructions live at once with nothing saying which is current.

## 1. Decision

`AGENTS.md` is the one real file. Every other entrypoint `init` touches is a
symbolic link to it (§FS-002-init.3).

`AGENTS.md` is the target because it is the convention that is not one vendor's:
the others are named for the tool that reads them, and picking any of those
would make every other tool's file a link to a competitor's. It is already this
spec's canonical fallback, so nothing is being introduced.

A symbolic link rather than a one-line pointer *to* `AGENTS.md`, because a
pointer is still a file whose content an agent has to follow, and following it
costs a read the link does not. A link is the same bytes at both paths; there is
no second file to upgrade, and no way for the two to disagree.

## 2. What this costs

- **Windows without Developer Mode refuses symbolic links.** `init` falls back
  to writing the block into the file and says so (§FS-002-init.3). Those
  repositories keep exactly today's behavior, duplication included.
- **`git config core.symlinks=false`** checks a link out as a text file holding
  its target path, which is instructions no agent can read. This is the same
  failure every symlinked repository already has, and it is a clone-time
  setting, not something `init` can see.
- **A companion with content of its own is not converted.** Overwriting it would
  destroy bytes §FS-002-init.4 promises to preserve, so it stays a regular file
  and the run reports it. Adoption covers the common case — the project that has
  a `CLAUDE.md` and no `AGENTS.md` — by making its content the canonical file.

## 3. What is not done

- **No rewriting of another tool's managed block.** A `CLAUDE.md` that also
  carries, say, a `grund` block is adopted whole: the content moves to
  `AGENTS.md` and both blocks travel with it. Splitting a file by which tool
  owns which region would need every one of those tools to agree first.
- **No link for a file `init` was not asked to touch.** Automatic mode's
  workspace triggers are unchanged; this decides what a touched entrypoint
  *is*, not which ones are touched.
