# E2E-062-remove-refuses-an-entry-that-still-silences: a working exception is not deleted

Removing an entry that is still silencing a finding repairs nothing — it reports
a file the repository decided to accept (§FS-009-exception-remove.3). A caller
acting on a stale-entry report is not asking for that, so the command measures
before it writes and refuses when a finding would appear that does not stand
today.

The refusal names the file, its measurement, and the limit that would report it,
and says what to do instead (§DF-007-instructions-at-the-error-site).
