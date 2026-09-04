# E2E-080-remove-reaches-an-orphan-shadow: the supported removal repairs the orphan

The hard entry has already been removed after `src/big.rs` shrank below the hard
limit, leaving the exact soft `shadows = "hard"` entry that the next strict load
rejects (§FS-003-exceptions.2.3). That rejection remains the contract for
`check` in E2E-054; it must not make the supported repair unreachable.

Addressing the orphan in the soft registry with `exception remove` deletes it,
reports the normal removal, and exits zero (§FS-009-exception-remove.2). The
version line, standalone registry note, and unrelated exception block remain
byte-for-byte where they were (§FS-009-exception-remove.4), leaving an ordinary
version-2 registry rather than requiring a hand edit.
