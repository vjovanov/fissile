# DF-010-stated-ceilings-are-exact: A ceiling the caller states is written as stated; only a measurement is quantized.

§DF-006-quantized-ceilings made every ceiling `fissile` writes a multiple of the
`[exceptions.bump]` step, so a registry records a decision — *this file may run
to 500 lines* — rather than the reading taken on the day the entry was written.
It applied the step to `--max` as well: the caller states a requirement, the
step chooses the number.

That half was wrong. A number typed on the command line is already a decision,
and rounding it replaces the caller's number with one nobody chose. Three
retunes of one file in five days said so in the tool log: the file grew by a
handful of lines each time, the step turned 436, 437, and 472 into 500 — the
rule's hard limit — `--max 480` could not hold it down, and each time the
registry was edited back by hand, past the validation `retune` exists to apply.
The glob form shows the same fault with no rule limit involved: the help text's
own example, `--max 300000 --unit bytes`, wrote 303104. A glob always takes
`--max`, so a glob ceiling could never be the round policy number someone
picked — and the entry §FS-005-exception-add.3 shows could not be produced.

## 1. Decision

The step quantizes measurements only. With `--max` omitted, `add` and `retune`
measure the file and write the smallest multiple of the step at or above it, as
§DF-006-quantized-ceilings.1 says. With `--max <N>`, both write `N`
(§FS-005-exception-add.2, §FS-008-exception-retune.1). The step still appears in
the result — as the next round number the caller could have chosen, named and
not applied.

This keeps what DF-006 was for. The fossil it retired is the measurement, and
the measurement is still never written as itself. What changes is that a stated
number is treated as the decision it is: a reviewer reading `480` in a registry
diff is reading a number a person or an agent typed and can be asked to defend,
which is exactly the property `500` was supposed to have.

## 2. A soft ceiling on the hard limit

A soft entry silences soft findings up to its ceiling, and a hard finding —
which fires at the hard limit — suppresses the soft finding on its own
(§FS-003-exceptions.3). A soft ceiling at or above the hard limit therefore
never fires: below the limit the entry silences the finding, at the limit the
hard gate does. When a rule's hard limit is a multiple of the step, that is
where the measured form lands every file in the last step below it — a
350/500-line rule with a 100-line step offers exactly one ceiling, 400, before
the step writes the hard limit itself.

So both commands refuse a soft ceiling at or above the hard limit for a file
still under it, whether the step produced it or the caller did (§FS-005-exception-add.4,
§FS-008-exception-retune.4). The refusal is the instruction
(§DF-007-instructions-at-the-error-site): it prints the form that would succeed
with the path, severity, and rule filled in — `--max <N> --unit lines` with the
range that is valid, or the hard `exception add` when the file belongs in the
other registry. An agent that has only ever run the plain form learns the stated
one at the moment it needs it.

The one exemption is a soft entry whose address the hard registry also holds.
Above the hard limit a deferred hard entry keeps the soft finding alive
(§FS-003-exceptions.3), so a soft ceiling there is doing work, and the twin is
what makes it legitimate.

A file already past the hard limit is the other case, and it is left alone. Its
soft entry is the record of debt §DF-008-hard-severity-needs-a-terminal.1 offers
an agent in place of the hard entry it may not write: dormant until a person
adds that entry, and from then on the twin that keeps it legitimate. Refusing it
would send the caller in a circle between two refusals.

## 3. What this costs

A stated ceiling grants exactly the headroom the caller asked for, which may be
none: `--max 472` on a 472-line file is the pinned ceiling DF-006 argued
against, and the next one-line edit trips it. That is the caller's call to
make, and the result says what the step would have written so the choice is
informed. The measured form remains the one agents reach for first, and it still
never writes a pinned number.

`audit`'s loose-ceiling report (§FS-003-exceptions.7) keeps the step as its
measure of slack. A stated ceiling within one step of its file is working room
whether the caller or the step chose it.

## 4. Rejected alternatives

- **Keep rounding `--max`; refuse only the hard-limit collision.** Leaves the
  glob case broken and still writes a number the caller did not type — and the
  refusal would have to offer a form that does not exist.
- **A `--exact` flag.** One more flag to know, for the behavior a typed number
  should have had. `--max` already says "this number".
- **Round `--max` down instead of up.** A number that is neither the
  measurement nor the caller's, and one that can fall below the file.
- **Clamp the measured form to just under the hard limit.** Writes `hard - 1`,
  a number nobody chose, and hides that the file is one step from a
  stop-the-line gate — the one place a per-file decision is worth asking for.

## 5. Consequences

- §DF-006-quantized-ceilings.1 and §DF-006-quantized-ceilings.5 are amended:
  the step governs measured ceilings, and `add` and `retune` share the rule
  rather than the number.
- Registries are untouched. A ceiling the old rule rounded stays where it is,
  and §FS-003-exceptions.4 accepts a soft ceiling above the hard limit as it
  always did — the refusal is on the write side, like the step itself.
- `retune --max` moves a ceiling by less than one step, which is how a rounded
  ceiling comes back down to the file it accepts.
- `audit --stale-exceptions` names the stated form for a loose soft entry
  whose step lands on the hard limit, instead of a `retune to` value the
  command would refuse (§FS-004-check-audit.2).
