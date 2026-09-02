# E2E-055-shadows-hard-takes-none-of-the-three: the flag replaces the three, it does not join them

`--shadows-hard` is not a shorthand that fills in missing flags: it says the
rationale lives in the other registry (§FS-005-exception-add.1.1). A `--reason`
passed beside it would be a second copy of an argument that already exists, in a
second file, free to drift from the first — the cost the flag was added to
remove (§FS-003-exceptions.2.3).

So the combination is a usage error rather than a precedence rule, and the
message names the flag to drop as well as the other way out: keep the reason and
drop the pointer. `--kind` and `--until` are refused on the same terms.
