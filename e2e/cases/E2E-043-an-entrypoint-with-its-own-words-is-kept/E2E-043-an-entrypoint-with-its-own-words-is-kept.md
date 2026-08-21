# E2E-043-an-entrypoint-with-its-own-words-is-kept: a link never destroys content

`CLAUDE.md` here holds a line `AGENTS.md` does not. Laying a link over it would
delete bytes §FS-002-init.4 promises to preserve, and `init` cannot know whether
the two entrypoints disagree on purpose (§FS-002-init.3).

So it stays a regular file, the managed block is written into it as before, and
the run reports `kept` rather than `linked` — the one word that tells the reader
this repository still has two files to keep in step, and why
(§DF-009-one-file-agents-read, §GOAL-003-friendly-output).
