# E2E-069-no-soft-ceiling-under-the-hard-limit-grants-headroom: the report says so rather than naming an empty range

`audit` names the stated form and a range when a soft entry's next step would
land on the hard limit (§FS-004-check-audit.2). For an entry with no headroom
that range starts one unit above the measurement, and when the file sits exactly
one unit under the hard limit the range is empty: there is no soft ceiling at
all that both grants headroom and still fires (§FS-003-exceptions.7).

A line printing `10 <= N < 10` would be advice nobody can take. This scenario
pins the other answer — the report says no soft ceiling helps and names the
route that does, the hard registry, in the words `retune`'s own refusal uses.
The run is still a report and still exits `0`.
