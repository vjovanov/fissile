# E2E-059-remove-repairs-a-registry-that-blocks-every-command: the entry that aborts every run can still be deleted

A rule limit raised above an entry's ceiling turns that entry into a load-time
failure (§FS-003-exceptions.4), and the abort is total: `check`, `audit`,
`measure` and the pre-commit hook all stop before measuring anything. Until
`exception remove` there was no command that could reach the entry, so the only
way out was hand-edited TOML.

`remove` loads without holding the entries to the rule check, because the entry
it deletes is what fails it (§FS-009-exception-remove.2). The registry it leaves
behind holds exactly the entries it read, less that one
(§FS-009-exception-remove.5).
