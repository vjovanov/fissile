# E2E-019-structural-silences-soft: a structural hard exception silences the soft warning too

The same fixture as `E2E-005-exception-silences-hard`, with one field changed: the
hard entry declares `kind = "structural"`. Splitting the file is illegal, so
there is nothing to minimize and the soft warning would name work nobody may do
— `fissile check` prints `ok` and nothing else (§FS-003-exceptions.3).

This is the half of the rule the kind field made knowable
(§DF-004-exception-kind.4). One entry makes the file quiet; before it, the only
remedy was a second entry in the soft registry repeating the same rationale
against a second ceiling free to drift.
