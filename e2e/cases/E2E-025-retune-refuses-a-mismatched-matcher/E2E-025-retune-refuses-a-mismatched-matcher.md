# E2E-025-retune-refuses-a-mismatched-matcher: a glob address does not retune an exact entry

An entry is addressed by its matcher, not just by a path that its matcher happens
to cover (§DF-005-exception-identity). A glob that merely spans an exact entry is
the wrong address in both of the ways that matter, and the command refuses rather
than writing (§FS-008-exception-retune.1).

It is the wrong address by intent: the caller asked for a class-wide ceiling, and
writing the number into one file's entry leaves every other file the glob names
at its old ceiling while reporting the change under a path no entry carries. And
it is the wrong address by arithmetic: a glob names no file to measure, so the
guard that keeps a ceiling from falling below the file it accepts has nothing to
compare against — here `--max 3` against a nine-line file, which would leave the
entry accepting less than the file it exists to accept
(§FS-008-exception-retune.2).
