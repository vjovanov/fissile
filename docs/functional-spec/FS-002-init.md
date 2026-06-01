# FS-002-init: fissile init installs config, exceptions, and agent instructions

`fissile init` bootstraps a repository for commit-time file-size discipline. It
writes the starter config file, creates the exception registries if requested,
and adds a versioned managed block to agent instruction files so coding agents
know how to react to soft and hard overflows. It follows the same non-intrusive shape
as `grund init`: preserve user-authored content, update only managed blocks, make
every choice a flag, and support `--dry-run`.

## 1. Inputs

```text
fissile init [<path>] [--name <name>] [--force] [--dry-run]
             [--config <path>] [--exceptions]
             [--agents-md] [--claude] [--gemini] [--copilot]
             [--cursor] [--windsurf] [--zed]
```

- `<path>` is the repository root to initialize. It defaults to `.` and must
  already exist.
- `--name <name>` is the human-readable project name used in a newly created
  `AGENTS.md` heading. It defaults to the target directory basename.
- `--config <path>` changes the config path written under `<path>`. The default
  is `.agents/fissile.toml`.
- `--exceptions` also creates the configured soft and hard exception registry
  paths when absent.
- `--force` refreshes managed agent blocks and generated starter files. It does
  not overwrite an existing config or any existing exception registries.
- `--dry-run` reports what would be written, appended, or updated without
  changing the filesystem.
- Agent flags explicitly select entrypoint families. Without any agent flag,
  automatic mode updates existing known entrypoints and otherwise creates
  `AGENTS.md`.

`init` is non-interactive. It never prompts and never guesses beyond the automatic
entrypoint selection rules in §3.

## 2. Files

Default `fissile init` writes:

- one agent entrypoint or managed block, per §3;
- `<path>/.agents/fissile.toml`, when absent, using the schema from
  §FS-001-config. The generated config is fully populated: every schema field is
  written at its default value, ready to edit in place, rather than a minimal
  skeleton (§DF-002-explicit-config).

With `--exceptions`, it also writes the configured exception registries, default
`docs/file-size-agent-exceptions.toml` and
`docs/file-size-human-exceptions.toml`, when absent. Each starter registry
contains `fissile_exceptions_version = 1`, explanatory comments, and no
exception entries.

Existing `.agents/fissile.toml` and existing exception registries are
project-owned. They are reported as `exists` and left byte-for-byte unchanged,
even with `--force`.

## 3. Agent Entrypoints

The canonical fallback is `<path>/AGENTS.md`. Automatic mode updates known
existing entrypoints instead of creating a competing canonical file. The known
set is:

- `AGENTS.md`
- `AGENTS.override.md`
- `CLAUDE.md`
- `.claude/CLAUDE.md`
- `GEMINI.md`
- `.github/copilot-instructions.md`
- `.cursor/rules/fissile.mdc`
- `.cursorrules`
- `.windsurfrules`
- `.rules`

Explicit flags create or update the requested families. Automatic mode creates
workspace-triggered aliases only when the matching tool-specific directory
already exists: `.claude/`, `.gemini/`, `.cursor/`, or `.zed/`. It does not
create `.github/copilot-instructions.md` merely because `.github/` exists, and
it does not create `.windsurfrules` or `.rules` without an explicit flag or
workspace signal.

## 4. Managed Block

The managed block heading is:

```markdown
## Keeping Files Small With fissile (v1)
```

That H2 line is the begin marker. The managed block runs until the next H1 or H2
heading, or end of file. A fresh `AGENTS.md` may have an unmanaged H1 above it;
companion entrypoints contain only the managed block unless they already had
user-authored content.

If an entrypoint has no managed block, `init` appends the current block. If it
has a supported block version, `init` replaces only the managed block and
preserves all bytes before and after it, including the block position. If it has
a newer unsupported block version, `init` exits with a schema error and leaves
the file unchanged.

The canonical v1 block teaches:

- run `fissile check --staged` before claiming work is done;
- treat soft overflows as agent-actionable guidance when the agent changed the
  file;
- treat hard overflows as stop-the-line failures unless a structured exception
  exists;
- read the message ID and guidance line as the configured remediation guidance
  for that rule;
- use `fissile exception add --severity soft` for accepted agent-facing warning
  debt;
- use `fissile exception add --severity hard` only for human-reviewed blocking
  debt;
- run `fissile audit --stale-exceptions` before removing or moving large files.

An example rendered block lives at `examples/AGENTS.fissile.md`.

## 5. Reporting

On success, stderr reports one path per line:

```text
wrote .agents/fissile.toml
appended AGENTS.md
```

Prefixes are `wrote`, `appended`, `updated`, and `exists`. Under `--dry-run`,
write prefixes become `would-write`, `would-append`, and `would-update`; `exists`
is unchanged. Stdout is empty.

After a run that wrote, appended, or updated something, stderr prints a short
`next:` block:

```text
next:
1. Review .agents/fissile.toml and tune rule limits.
2. Install the pre-commit hook that runs fissile check --staged.
3. Run fissile audit once and add justified exceptions with fissile exception add.
see AGENTS.md for the full workflow.
```

The `next:` block is suppressed when every selected file already exists with the
current managed block.
