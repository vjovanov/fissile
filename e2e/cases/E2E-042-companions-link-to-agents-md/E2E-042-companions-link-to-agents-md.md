# E2E-042-companions-link-to-agents-md: one file, read from every path

`--claude` asks for `CLAUDE.md`, and `AGENTS.md` is written whichever families
are asked for, because a link has to point at something (§FS-002-init.3).
`CLAUDE.md` is a symbolic link to it, so the block exists once and every agent
reads the same bytes (§DF-009-one-file-agents-read, §GOAL-004-token-thrift).

The link target is asserted as stored — `AGENTS.md`, relative — not resolved:
an absolute target would break the moment the repository is cloned somewhere
else, which is the whole reason it is relative.
