# E2E-014-init-names-entrypoint: the next block names the entrypoint it wrote

`init` updates whichever agent entrypoints already exist and only falls back to
`AGENTS.md` when none do (§FS-002-init.3). The closing `see <path> for the full
workflow.` line must therefore name what this run actually handled: in a
repository carrying `CLAUDE.md` and no `AGENTS.md`, sending the reader to
`AGENTS.md` would point at a file that does not exist, at exactly the moment
they are looking for the workflow they just installed (§FS-002-init.5,
§GOAL-003-friendly-output).
