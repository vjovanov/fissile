# E2E-007-init-installs-hook: init bootstraps config, registries, agent block, and the hook

In a git repository, `fissile init --exceptions` writes the starter config and
exception registries, adds the managed agent block, and installs the pre-commit
hook that runs `fissile check --staged`. It bootstraps a repo for commit-time
discipline (§FS-002-init) including the managed hook (§FS-002-init.6).
