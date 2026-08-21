# E2E-040-json-check-explains-its-exit-code: a JSON run never fails silently

`check --format json` can exit non-zero for something that is not an overflow: a
dead exception entry under `[exceptions].stale = "error"` (§FS-004-check-audit.1.3).
The findings array is the stable machine contract and stays exactly that, so the
account of what happened goes to stderr, which already owns every diagnostic a
JSON run emits (§FS-004-check-audit.5).

What this rules out is the shape a CI job cannot act on: zero findings on stdout
and a failing exit code with nothing anywhere saying why.
