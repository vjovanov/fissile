# DF-006-quantized-ceilings: A recorded ceiling is a round number, not a measurement.

`fissile exception add` wrote a file's exact measurement as `max_accepted`, and
nothing could move it afterwards. Both halves were wrong, and they were wrong
together: a ceiling pinned to the byte is re-crossed by the next small edit, and
with no command to move one, every crossing ended in hand-edited registry TOML —
past the validation `exception add` exists to apply.

A ceiling exists to stop drift, not to price a file. Pinned to the exact
measurement it does neither job well. A two-line edit trips it, the correct
response is a two-line raise, and the registry fills with numbers nobody chose:
`486`, then `488`, then `491`, each one a fossil of the commit that happened to
touch the file.

## 1. Decision

Ceilings are quantized. `[exceptions.bump]` sets one step per unit — 100 lines,
4096 bytes, 1000 tokens by default (§FS-001-config.5) — and every ceiling
`fissile` writes is the smallest multiple of that step at or above the value it
has to accept (§FS-005-exception-add.2). A 488-line file is accepted at 500.

`fissile exception retune` moves a recorded ceiling to that same quantized value
(§FS-008-exception-retune), so a ceiling that has to move is moved by the tool
that computes it, in one command, with the registry revalidated afterwards.

The number in the registry is then a decision — *this file may run to 500 lines*
— rather than a reading. A reviewer can disagree with a decision.

## 2. Why the step also defines "too loose"

Quantization grants slack on purpose. A report that told the reader to remove
that slack would be arguing with the number `fissile` had just written, so the
step settles both questions at once: slack within one step is the working room
the bump granted, and slack beyond one step is the ratchet slipping
(§FS-003-exceptions.7).

Without that second half the ratchet only ever turns loose. A file shrinks when
its module is finally split, the ceiling stays where it was, and the budget the
split paid back is silently re-granted to whoever edits the file next. One number
in the config governs both directions, so the two can never contradict.

## 3. What this costs

A quantized ceiling accepts lines nobody has written yet. That is the honest
price, and it is smaller than it looks: the *budget* is the rule's soft and hard
limits, which quantization does not touch. The ceiling is a ratchet against
drift on top of an already-accepted file, and a coarser tooth still ratchets — it
just stops the tool from billing an agent for two lines.

## 4. Rejected alternatives

- **A percentage bump.** One number covers every unit, but it yields 536, 1674,
  291 — numbers that look measured, which is the reading being retired here. It
  also grows without bound on the largest files, exactly where the ratchet most
  needs to bite.
- **A grace band on the limit itself.** Letting a rule's limit be crossed by *n*
  before it reports would change what a limit means, in every repository, for
  files that carry no exception at all. The problem is the ceiling, so the fix
  belongs to the ceiling (§GOAL-006-graded-limits).
- **Exact ceilings plus a raise command.** Keeps `max_accepted` honest to the
  byte and still lets the tool move it. But it relocates the churn instead of
  removing it: every small edit to an accepted file still produces a registry
  diff, and a reviewer still cannot tell a deliberate ceiling from an accident of
  timing.

## 5. Consequences

- `add` and `retune` write the same quantized value, so which command created an
  entry is not visible in the result.
- Registries written before this decision keep working. Quantization is a
  write-side policy; §FS-003-exceptions.4 still accepts any ceiling at or above
  the rule limit, so no existing entry becomes a schema error.
- A repository that wants the old behavior sets the step for its units to `1`.
- The managed agent block tells agents to let `retune` choose the number rather
  than picking a ceiling by hand (§FS-002-init.4), because an agent asked for a
  ceiling picks the smallest one that clears the file — which is the behavior
  this decision exists to end.
