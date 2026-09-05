# E2E-081-the-config-home-governs-the-run: the config at `.agent-grounds/` is the one in force

A repository whose only config is `.agent-grounds/fissile.toml` is governed by
it (§FS-001-config.8.1). That is the whole of the move this scenario exists for:
before it, the same tree ran on the built-in defaults, enforcing four generic
rules and none of the ones its author wrote.

The proof goes through `limits`, which enumerates the configured rules and
reads no other document (§FS-010-limits.2, §FS-010-limits.5). One rule with a
name no default carries makes the answer unambiguous — the output either names
the tree's own rule or it names the defaults, and there is no third reading. A
`check` would have proved the same thing more expensively and through a
measurement the scenario does not care about.

Nothing here is deprecated, so nothing is warned about: the stdout comparison is
exact, and stderr carries no line this scenario has to name.
