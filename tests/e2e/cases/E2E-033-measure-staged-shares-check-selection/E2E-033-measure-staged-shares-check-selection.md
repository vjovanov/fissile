# E2E-033-measure-staged-shares-check-selection: measure --staged selects what check --staged selects

`measure` and `check` are different commands under different exit contracts, but
they answer one question the same way: which files am I looking at
(§FS-007-measure.1). `--staged` takes the set from git and applies
`[scan].exclude`, so the generated file staged alongside the source is dropped
from both.

The selection and the measurement behind it are one piece of code, not two that
happen to agree today (§FS-006-cli.1). This scenario is what says so: a change to
staged measurement that reached only one of the two commands stops here.
