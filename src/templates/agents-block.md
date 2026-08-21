<!-- BEGIN FISSILE MANAGED BLOCK -->
## Keeping Files Small With fissile (v3)

This repository caps file size with [`fissile`](https://github.com/vjovanov/fissile)
so that agents spend fewer tokens reading. A pre-commit hook runs
`fissile check --staged`, and its findings say what to split and how. Run it
yourself before claiming work is done, and never get past it with `--no-verify`.
<!-- END FISSILE MANAGED BLOCK -->
