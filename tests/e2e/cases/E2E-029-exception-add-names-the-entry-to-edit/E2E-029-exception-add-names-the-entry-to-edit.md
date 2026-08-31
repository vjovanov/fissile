# E2E-029-exception-add-names-the-entry-to-edit: a glob over two entries is a conflict, not a broken registry

Two exact entries for two different files is a registry §FS-003-exceptions.4
accepts: neither answers the other's condition, and nothing about it is
ambiguous. A glob covering both of them is an address the caller chose, and the
refusal has to be about that choice.

`exception add` reports what it always reports — the entry already accepting this
condition, named by where it lives, and `fissile exception retune` as the command
that moves it (§FS-005-exception-add.4). Telling the caller instead that "the
registry has to name one entry per condition" would send them to edit a file that
is correct.
