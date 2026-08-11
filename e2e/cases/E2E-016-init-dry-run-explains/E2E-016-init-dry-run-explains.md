# E2E-016-init-dry-run-explains: a dry run explains the workflow without writing

`fissile init --dry-run` prints the managed agent block on stdout while the
planned writes stay on stderr, and no file is created (§FS-002-init.4,
§FS-002-init.5). This is where `fissile --help` sends a reader who wants more
than the usage paragraph (§FS-006-cli.2), so it is the answer for an agent
dropped into a repository whose entrypoint never received the block: what the
tool expects of it, readable without touching the working tree.
