# E2E-052-a-shadowing-twin-silences-the-soft-finding: the pair reads back as one accepted file

`E2E-005-exception-silences-hard` is the state this repository starts from
without the twin: a deferred hard entry accepts the file and the soft warning
survives it (§FS-003-exceptions.3). Adding the shadowing soft entry is what
finishes the job.

The registry here states no `reason` and no `until` in the soft entry. Both are
resolved from the hard entry at the same address before anything is evaluated
(§FS-003-exceptions.2.3), so the entry is a complete one by the time the soft
finding is judged against it, and `check` prints `ok`.
