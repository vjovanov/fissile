# E2E-020-registry-version-2: an unmigrated registry is refused with both edits named

`id` and `replaces` are gone from the schema, and version 2 removes them rather
than tolerating them: a registry that still declares version 1 is rejected, and a
version-2 registry that still carries `id` fails on the unknown key
(§FS-003-exceptions.2.2, §DF-005-exception-identity).

The fixture is a version-1 registry, exactly what every adopter has on the day
they upgrade. What it pins is the error text, not the refusal: the message names
the file, then both edits — bump the version line, delete the `id` and `replaces`
lines. An upgrade error that only states which version the build supports leaves
the reader to work out the remedy, which is the failure
§GOAL-003-friendly-output.1 exists to prevent.
