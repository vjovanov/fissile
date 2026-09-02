# E2E-064-audit-names-remove-for-an-entry-that-silences-nothing: the report names the command, not just the state

A loose ceiling is normally retuned down (§FS-008-exception-retune.2), but an
entry whose file has fallen under the rule's limit has no ceiling worth writing:
it silences nothing, and the way out is to delete it (§FS-003-exceptions.7).

`retune` refuses that lowering and says so (E2E-030). This scenario pins the
other half — the report that finds the entry in the first place names
`fissile exception remove` (§FS-009-exception-remove), so the reader is not left
holding a diagnosis with no command.
