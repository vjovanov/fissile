## Keeping Files Small With fissile (v1)

This project uses `fissile`: every commit should keep changed files under their
configured budgets. Run `fissile check --staged` before claiming work is done,
or rely on the pre-commit hook when it is installed.

### When fissile Reports A File

Findings are grouped. A block has three parts:

- a header naming the severity, the crossed limit, the rule, and the message ID;
- one project-owned guidance passage, printed once, that applies to the whole
  block;
- the files it applies to, largest first.

If you changed a reported file in this turn, follow the configured guidance and
try to bring it back under the limit. If you did not change it, leave it alone
unless the task is about that file.

A **soft** overflow means *should split*: do it along a seam the code already
has, not at the line count, and never by breaking apart things that belong
together. A **hard** overflow means *must split*: it blocks the commit.

### Exceptions

Never damage the design to fit a budget — that is what the registries are for.

Soft-limit exceptions are agent-facing warning debt. If a file is intentionally
above the soft limit and no split leaves the code better, run
`fissile exception add <path> --severity soft --rule <id> --reason <text> --until <text>`.

Hard-limit overflows are not bypassed with flags or source comments. If you
cannot see a split that keeps the architecture intact, ask a human: a
human-reviewed
`fissile exception add <path> --severity hard --rule <id> --reason <text> --until <text>`
is the only other way past the gate.

Use `fissile audit --stale-exceptions` before removing or moving large files so
dead exceptions do not stay in the registries.
