# E2E-028-measure-agrees-at-a-ceiling: a file exactly at its ceiling is accepted, and reads that way

An exception silences *at* its ceiling: `max_accepted = 9` accepts a nine-line
file, and `check` prints `ok` for it (§FS-003-exceptions.3). A limit is the other
way round — it fires at the limit — and `measure` reports both on one line, so
the two arithmetics have to be kept apart (§FS-007-measure.2).

Read as if a ceiling behaved like a limit, this file is `0 over soft-accepted`,
rendered as an overflow in warning colour, for a file the gate accepts. It has
zero room left, which is what the clause says: `0 to soft-accepted`.
