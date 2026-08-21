# DF-008-hard-severity-needs-a-terminal: A hard exception is refused off a terminal, and `--force` is the way past.

A hard exception is the only way past a stop-the-line gate, so §DF-003-severity-guidance.1
holds that it is not something an agent grants itself. Until now that was said
and never enforced: the shipped hard message asks the reader to stop and ask a
human, and `fissile exception add --severity hard` then runs for whoever typed
it. An agent that could not find a split had one command that made the gate go
away, and nothing between it and that command.

## 1. Decision

`fissile exception add --severity hard` refuses when standard input is not a
terminal, and names the soft-severity route in the refusal (§FS-005-exception-add.4).
`--force` proceeds anyway.

Standard input is the signal because it separates the two callers as they
actually run: a person adding an exception is at a terminal, and an agent
shelling out through a tool, a hook, or CI is not. It costs nothing, needs no
new flag in the common case, and is wrong only in the direction that asks a
human to type one more word.

The refusal is a speed bump, not a lock, and is not designed as one. `--force`
is documented and undefended: an agent that reads the error can pass it. What
changes is that granting itself the exception stops being the path of least
resistance and starts being a flag whose name says what it is doing.

## 2. What is not added

- **No provenance field on the entry.** Recording `added_by = "agent"` would
  make `--force` visible in review, at the cost of a §FS-003-exceptions.2 schema
  field that every registry reader must then carry. The hard registry is the
  human-reviewed file by construction (§FS-003-exceptions.6): a new entry in it
  is already a diff someone reads, and the reason it carries is already the
  claim they weigh (§GOAL-007-justified-exceptions).
- **No gate on `retune`.** Moving a hard ceiling is bookkeeping on a decision a
  human already made, which is what §FS-008-exception-retune.2 says separates it
  from adding one. Gating it would push callers back to hand-edited registry
  TOML, the outcome that command exists to prevent.
- **No gate on soft.** Soft severity is the agent's to record — that is what
  makes it the honest alternative this refusal can offer.

## 3. Rejected alternatives

- **An interactive confirmation prompt.** §FS-002-init states `init` never
  prompts, and the same holds here: a command that blocks on a read is a command
  that hangs in every non-interactive caller, including the hook.
- **An environment variable instead of a flag.** A variable set once in a shell
  profile or a CI job stops being a decision and disappears from the command
  that made it. A flag is in the shell history and in the transcript.
- **Refusing outright with no escape.** Scripted repositories add hard entries
  legitimately — a migration accepting a vendored tree, a `Makefile` target a
  human runs. Leaving them no route makes hand-edited registry TOML the answer,
  which skips every check the command applies (§FS-008-exception-retune).

## 4. Consequences

An agent that hits a hard overflow it cannot split now has one route that works
without a flag — record the debt at soft severity, which leaves the finding
standing rather than silencing it — and one that names itself. Repositories
scripting hard entries add `--force` once, in the script, where a reader sees
it.
