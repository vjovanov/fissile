# E2E-057-a-structural-twin-leaves-the-soft-ceiling-dead: a pointer is not a licence

This is E2E-056's repository with `kind = "structural"` in place of
`kind = "deferred"`. Everything else — the 3-line file, the hard limit of 4, the
`--max 10` asked for, the flags — is identical.

A structural hard entry ends evaluation for the overflow it accepts
(§FS-003-exceptions.3), so a soft ceiling above the limit would never fire, and
it is refused on the same terms as one with no hard entry at all
(§DF-010-stated-ceilings-are-exact.2). `--shadows-hard` does not buy past that:
the flag says where the rationale lives, and it guarantees a hard entry at the
address rather than a deferred one (§FS-005-exception-add.1.1).

The refusal is the ordinary one, which means the command it offers is this call
with `--shadows-hard` still in it and a ceiling under the limit — it runs as
printed (§FS-005-exception-add.4). It offers no hard-severity route, because the
hard entry is the thing this call points at (§FS-005-exception-add.1.1). Nothing
is written.
