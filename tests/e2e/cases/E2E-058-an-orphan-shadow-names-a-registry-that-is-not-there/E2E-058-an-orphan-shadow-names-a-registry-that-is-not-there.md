# E2E-058-an-orphan-shadow-names-a-registry-that-is-not-there: the file to open is named either way

E2E-054 deletes the hard entry and leaves its registry on disk. This is the
other way a twin ends up pointing at nothing: the hard registry was never
written, which is what a repository looks like when someone hand-writes
`shadows = "hard"` before recording the acceptance it means to shadow.

The refusal is the same schema error (§FS-003-exceptions.4) and it names the
same file: `docs/file-size-human-exceptions.toml`, the configured hard registry,
whether or not that path exists yet (§FS-003-exceptions.2.3). Naming the concept
instead — "the hard registry" — would send a reader with a non-default
configuration looking for the wrong file, when the whole point of leading a
diagnostic with the registry path is that it is the line to edit
(§DF-005-exception-identity).

Both ways out still stand: write the hard entry whose reason and until this one
would inherit, or drop the pointer and state them here.
