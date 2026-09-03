# E2E-068-a-spent-soft-ceiling-names-the-range-under-the-hard-limit: the range starts above the measurement

A soft ceiling at or above the rule's hard limit never fires for a file still
under that limit, so `fissile exception retune` refuses it and names the stated
form instead (§FS-008-exception-retune.4). `audit` names the same form rather
than a value the command would decline (§FS-004-check-audit.2), which E2E-049
pins for a loose entry.

An entry with no headroom needs one thing more: the range has to start *above*
the measurement. A ceiling at the measurement is exactly what the entry already
records, so offering it back would name a retune that leaves the entry as spent
as it found it (§FS-003-exceptions.7). Here the file measures 9 and the hard
limit is 12, so the range the line prints is `10 <= N < 12`.
