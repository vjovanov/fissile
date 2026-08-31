# E2E-036-staged-check-names-the-gate: a blocked commit says so, and says not to bypass it

`check --staged` is the commit gate, so a standing hard overflow there closes
with what the finding's own guidance cannot say: the commit is blocked, a
reviewed hard exception is the other way through, and `--no-verify` only moves
the overflow into the branch (§FS-004-check-audit.1.2).

Only `--staged` prints it. The same findings from CI or from `fissile check
src/` are not blocking anything a caller is about to bypass, which is why
`E2E-002-check-hard-blocks` sees the findings and the hint and no epilogue.
