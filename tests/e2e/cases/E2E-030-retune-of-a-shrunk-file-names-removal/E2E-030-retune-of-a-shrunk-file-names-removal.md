# E2E-030-retune-of-a-shrunk-file-names-removal: a file under its limit cannot be followed down

Lowering a ceiling is how a loose one is retired (§FS-008-exception-retune.2),
but it stops at the rule's limit: below that the entry would silence nothing, and
the way out is to delete it. `audit --stale-exceptions` already says exactly that
about this entry (§FS-003-exceptions.7), so the refusal on the documented path
says it too.

The remedy names the command that performs it, addressed exactly as the caller
addressed the entry, so the offered call runs as printed
(§FS-009-exception-remove.1, §DF-007-instructions-at-the-error-site).

The caller passed no `--max`, so the error blames no `--max`. It reports the
measurement `retune` actually read and the file it read it from, because that is
the fact the caller has to act on.
