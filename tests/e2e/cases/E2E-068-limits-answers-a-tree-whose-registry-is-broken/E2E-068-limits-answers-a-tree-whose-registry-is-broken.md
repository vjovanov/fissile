# E2E-068-limits-answers-a-tree-whose-registry-is-broken: the config is readable while the tree is not

This is the tree of §E2E-054-an-orphan-shadow-fails-the-load: a `shadows = "hard"`
entry pointing at a hard entry that is gone, which is a schema error the
registry load refuses (§FS-003-exceptions.4). Every command that validates the
registries exits `2` here, `check` and `audit` alike.

That is the moment a reader most needs to know what the tree enforces, and it is
why `limits` loads the config and nothing else (§FS-010-limits.5). It prints the
full inventory and exits `0`. A `limits` that reached for the shared load would
pass this case only until a repository broke a registry — which is the one state
it exists to survive.
