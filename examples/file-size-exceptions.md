# File Size Exceptions

Oversized files accepted by this repository. Each exception is a grund declaration
with a stable `EX-` ID, a rationale paragraph, and structured fields.

## EX-001-generated-parser-fixture: generated parser fixture

`tests/fixtures/parser/large-corpus.json` is intentionally large because it is a
golden corpus copied from production parser incidents. Retire this exception when
the fixture can be generated deterministically inside the test or split by parser
feature without losing incident coverage.

- **Path:** `tests/fixtures/parser/large-corpus.json`
- **Match:** exact
- **Rules:** `fixtures`
- **Limit waived:** hard
- **Until:** review after parser fixture generator lands
- **Owner:** `parser`
- **Created:** `2026-05-26`
