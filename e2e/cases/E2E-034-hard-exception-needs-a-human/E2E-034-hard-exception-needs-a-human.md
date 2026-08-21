# E2E-034-hard-exception-needs-a-human: a hard exception is refused off a terminal

`fissile exception add --severity hard` is the only command that makes a
stop-the-line gate go away, so it is refused when standard input is not a
terminal (§FS-005-exception-add.4, §DF-008-hard-severity-needs-a-terminal.1).

The refusal is checked for both halves of what it owes the caller: the route an
agent can take on its own — the same entry at soft severity, which leaves the
finding standing — and the flag a script legitimately uses. A refusal that named
neither would leave hand-edited registry TOML as the only way forward, which is
the outcome `fissile exception add` exists to prevent.
