# DA-001-token-external-command: token counting shells out; no tokenizer is bundled.

`fissile` supports a `tokens` unit so a budget can be expressed in the currency it
ultimately cares about — model context (§GND-001-fissile). But the default binary
embeds no tokenizer. Token mode is opt-in and, when on, counts by running a
project-configured external command, not by linking a tokenizer into `fissile`.

## 1. Decision

The `[tokens]` config block carries `enabled` (default `false`) and a `command`
template (§FS-001-config.7). With token mode off, files are measured in bytes and
lines only, needing no external tooling. With it on, `fissile` invokes the
configured command per file and parses its count; it never carries tokenizer model
data of its own. The unit is first-class in the engine and output (`Unit::Tokens`),
so the measuring and reporting paths do not special-case it — only the *acquisition*
of the count is external.

## 2. Why

A bundled tokenizer would break two goals at once.

- **Footprint.** Tokenizer model files are large and tokenizer crates pull a wide
  dependency tree. Either would blow the single-small-static-binary contract that is
  the adoption promise (§GOAL-002-tiny-footprint.1). Token mode is the one feature
  whose natural implementation is heavy, so it is the one feature held at arm's
  length.
- **Honesty about which tokenizer.** "Tokens" is not one number — it depends on the
  model family. Baking in one tokenizer would silently privilege one vendor's count
  and mislead every project on a different model. Delegating to a project-named
  command lets the repo measure against the tokenizer it actually ships to.

This also keeps configuration as data, not code (§GOAL-002-tiny-footprint.2): the
project supplies a command line, not a plugin loaded into `fissile`'s process.

## 3. Consequences

- The default build stays small; nothing about token support links into a repo that
  does not enable it (§GOAL-002-tiny-footprint.3).
- Token mode adds a process spawn per measured file. That is acceptable because it is
  opt-in and a project can scope it to the file types that need it via rules and
  exclusions, keeping the hot path fast for everyone else (§GOAL-001-fast-feedback,
  §GOAL-005-configurable.3).
- A misconfigured or missing token command is a clear runtime error on the files it
  was asked to measure, not a silent fallback to a different unit.
- If a future build ever embeds a tokenizer, it must be behind a non-default build
  feature so the default artifact's size contract is preserved; this record is the
  place to revisit that.
