# E2E-056-a-deferred-twin-lifts-a-soft-ceiling-over-the-hard-limit: the kind of the entry shadowed is what decides

A soft ceiling at or above a rule's hard limit is normally refused: the hard
finding takes over up there, so the soft one would never fire
(§DF-010-stated-ceilings-are-exact.2). The exemption is a *deferred* hard entry
at the same address, which leaves the soft finding standing
(§FS-003-exceptions.3) and so gives the ceiling something to silence.

`--shadows-hard` guarantees a hard entry at the address, not a deferred one
(§FS-005-exception-add.1.1), so this is the shape where the two come apart. The
file is 3 lines against a hard limit of 4 — under the limit, and therefore not
spared by the exemption a file already past it carries (§FS-005-exception-add.4)
— and the ceiling asked for is 10, well above the limit. The entry it shadows is
`deferred`, and the command writes the twin.

E2E-057 is the same repository with one word changed in the hard registry, and
it is refused. That pair is the whole rule: what decides here is the kind of the
entry being shadowed, and nothing else in the call.
