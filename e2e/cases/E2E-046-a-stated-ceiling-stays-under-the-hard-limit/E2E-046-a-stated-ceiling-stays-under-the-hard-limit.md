# E2E-046-a-stated-ceiling-stays-under-the-hard-limit: the stated form is the way through the last step below the limit

The same repository as §E2E-045-retune-refuses-a-soft-ceiling-on-the-hard-limit,
with the ceiling stated: `--max 6` on a 5-line file under a hard limit of 8 is
written as 6 (§FS-008-exception-retune.1, §DF-010-stated-ceilings-are-exact.1).

The result carries no `next step` suggestion. The step's next multiple is 8, the
hard limit, which is a ceiling the command refuses — naming it would send the
caller straight into that refusal (§FS-008-exception-retune.3).
