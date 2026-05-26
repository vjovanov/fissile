## Keeping Files Small With fissile (v1)

This project uses `fissile`: every commit should keep changed files under their
configured budgets. Run `fissile check --staged` before claiming work is done,
or rely on the pre-commit hook when it is installed.

### When fissile Reports A File

A finding has two parts:

- a stable machine-readable line naming the file, unit, limit, rule, and message ID;
- one short project-owned guidance line explaining how this repository wants the
  file split.

If you changed the reported file in this turn, follow the configured guidance and
try to bring it back under the soft limit. If you did not change it, leave it
alone unless the task is about that file.

### Exceptions

Hard-limit overflows are not bypassed with flags or source comments. If a file is
large for a real reason, add a written exception to `docs/file-size-exceptions.md`
and cite that exception from the file or architecture that depends on it.

Use `fissile audit --stale-exceptions` before removing or moving large files so
dead exceptions do not stay in the registry.
