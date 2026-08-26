# E2E-045-retune-refuses-a-soft-ceiling-on-the-hard-limit: the measured form stops where a soft ceiling would stop firing

A soft entry silences soft findings up to its ceiling, and at the hard limit
the hard finding takes over and suppresses the soft one (§FS-003-exceptions.3).
A soft ceiling at or above that limit therefore never fires. The rule here is
2/8 lines with a 4-line step, and the file has grown to 5: the step's next
multiple is 8, the hard limit itself.

`retune` refuses rather than writing a dead ceiling, and the refusal is the
instruction (§FS-008-exception-retune.4, §DF-010-stated-ceilings-are-exact.2):
it prints this call with `--max <N> --unit lines` and the range `N` may take,
so an agent that has only ever run the measured form learns the stated one at
the moment it needs it (§DF-007-instructions-at-the-error-site). Nothing is
written: the entry still records 4.
