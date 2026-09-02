# E2E-061-remove-deletes-a-stale-entry: the entry whose file is gone is the one removal is for

`check` and `audit` both report an entry that accepts a file that is not there
and both say to remove it (§FS-004-check-audit.1.3). The entry silences nothing,
so nothing surfaces when it goes (§FS-009-exception-remove.3).

The scenario also pins what stays: the entry beside it keeps its ceiling and its
reason, because a removal deletes one block and preserves every other byte
(§FS-009-exception-remove.4).
