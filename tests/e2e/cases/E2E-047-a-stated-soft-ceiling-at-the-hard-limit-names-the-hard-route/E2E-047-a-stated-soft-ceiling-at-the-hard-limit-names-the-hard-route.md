# E2E-047-a-stated-soft-ceiling-at-the-hard-limit-names-the-hard-route: a stated value gets the same refusal, plus the other registry

Stating the number does not make a dead ceiling live: `--max 8` for a soft
entry under a hard limit of 8 is refused like the measured form is
(§FS-008-exception-retune.4, §DF-010-stated-ceilings-are-exact.2).

A caller who typed a number at the hard limit may have meant the other
registry, so this refusal offers both routes: the same call with `--max <N>`
and the range that stays under the limit, and the hard-severity `exception add`
that accepts the file where it is heading. Nothing is written.
