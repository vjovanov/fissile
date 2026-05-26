# Functional spec

External behavior — *what* this project does. One file per spec; each H1 declares an `FS-NNN-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§FS-NNN-<slug>.<section>`) resolve into these files.

By convention every spec under this directory is linked from this README so the index stays a complete table of contents. Extra prose, recommended reading order, and conceptual groupings are welcome around the link set.

| ID | Subject |
|---|---|
| [§FS-001-config](FS-001-config.md#fs-001-config-fissile-reads-a-versioned-toml-config-file) | fissile reads a versioned TOML config file |
| [§FS-002-init](FS-002-init.md#fs-002-init-fissile-init-installs-config-exceptions-and-agent-instructions) | fissile init installs config, exceptions, and agent instructions |
| [§FS-003-exceptions](FS-003-exceptions.md#fs-003-exceptions-oversized-files-are-accepted-through-a-cited-registry) | oversized files are accepted through a cited registry |
| [§FS-004-check-audit](FS-004-check-audit.md#fs-004-check-audit-fissile-check-and-audit-enforce-file-budgets) | fissile check and audit enforce file budgets |

This index is navigational — citations should target the spec ID directly, never this file.
