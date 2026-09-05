# E2E-094-audit-only-refuses-a-json-config: the refusal follows the format, not the flag

`--only` with `--format json` is a usage error rather than a filter, and a
config's `[output].format` reaches JSON by a different road than the flag does
(§FS-004-check-audit.2). This repository sets `format = "json"` and passes no
`--format` at all, so the command line carries nothing to refuse and the
argument parser lets the run through.

The refusal has to happen where the format is finally known — after the config
is loaded — or the selection would be accepted and quietly ignored. That is the
failure the spec singles out: a caller who cannot tell a selection that was
honoured from a section that had nothing to say. Here the run exits `2` and
names both flags, and prints no JSON object, so the answer is the same one the
command line already gives.

E2E-092 pins the other road, where `--format json` is typed and the argument
parser refuses before any config is read. Two roads to one format need two
refusals, and one case each.
