# E2E-010-cli-version: --version prints one stable line

`fissile --version` prints exactly `fissile <semver>` on stdout and exits `0`
(§FS-006-cli.3): no banner, no build metadata, one token-cheap line
(§GOAL-004-token-thrift). The pinned bytes double as the release self-check —
every published binary must answer with the version being released
(§AR-001-ci.8).
