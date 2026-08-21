<!-- >>> fissile managed block (v3) >>> -->
## Keeping Files Small With fissile

This repository caps file size with [`fissile`](https://github.com/vjovanov/fissile)
so that agents spend fewer tokens reading. A pre-commit hook runs
`fissile check --staged`, and its findings say what to split and how. Run it
yourself before claiming work is done, and never get past it with `--no-verify`.
<!-- <<< fissile managed block (v3) <<< -->
