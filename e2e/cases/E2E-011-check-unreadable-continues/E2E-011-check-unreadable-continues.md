# E2E-011-check-unreadable-continues: one unmeasurable path never hides the rest

A path that cannot be measured is skipped with a stderr line that names it,
while every other file is still measured and its findings print normally
(§FS-004-check-audit.5). The run exits `2` — passing a file the gate could not
read would be unsound — but the hard finding on the readable file is still
delivered, so the contributor fixes both problems from one run.
