# E2E-060-audit-passes-after-a-removal: the repaired registry loads again

The point of `exception remove` is not the write, it is what the repository can
do afterwards (§FS-009-exception-remove.2). This scenario starts from the
registry E2E-059 leaves behind and shows the command that was aborting — `audit`
— running to completion, with nothing stale left to report.
