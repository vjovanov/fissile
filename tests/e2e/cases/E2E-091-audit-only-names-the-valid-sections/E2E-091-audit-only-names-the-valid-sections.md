# E2E-091-audit-only-names-the-valid-sections: a name that is not a section is not an empty report

The seven section names come from `schema/audit.schema.json`, which is what
keeps the text surface and the JSON surface from growing two vocabularies for
one report (§FS-004-check-audit.2). A name outside that set is a usage error,
exit `2`, with the diagnostic on stderr like every other run-level failure
(§FS-004-check-audit.5).

What it must not be is a silent empty report. A section that is genuinely empty
prints nothing too — the `exceptions:` counts omit themselves when both
registries are empty — so a typo answered with silence is indistinguishable from
a true answer, and the caller reads the wrong one.

The message names both halves the caller needs: the name that was not
recognized, and the whole set that would have been, so the fix is a choice from
the line in front of them.
