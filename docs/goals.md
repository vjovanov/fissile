# Goals

What this checker measures itself against. If a change does not advance one of these, it is not worth doing. Goals are declared inline below; each is a stable ID and may be cited from anywhere in the repo.

Current goals:

- [§GOAL-001-fast-feedback](goals.md#goal-001-fast-feedback-the-hook-is-imperceptible)
- [§GOAL-002-tiny-footprint](goals.md#goal-002-tiny-footprint-one-small-static-binary-no-runtime)
- [§GOAL-003-friendly-output](goals.md#goal-003-friendly-output-tell-the-user-exactly-what-broke-and-how-to-fix-it)
- [§GOAL-004-token-thrift](goals.md#goal-004-token-thrift-the-tool-itself-spends-as-few-tokens-as-it-saves)
- [§GOAL-005-configurable](goals.md#goal-005-configurable-every-limit-and-message-overridable-per-file-type-and-path)
- [§GOAL-006-graded-limits](goals.md#goal-006-graded-limits-soft-warns-hard-blocks-ai-minimizes)
- [§GOAL-007-justified-exceptions](goals.md#goal-007-justified-exceptions-every-oversized-file-has-a-written-reason)
- [§GOAL-008-architecture-aware-messages](goals.md#goal-008-architecture-aware-messages-overflows-explain-the-local-architecture)

## GOAL-001-fast-feedback: the hook is imperceptible

A pre-commit hook is run by a human waiting on a prompt. If it is slower than a heartbeat the human routes around it — `--no-verify`, uninstall, or quiet resentment — and the protection [§GND-001-fissile](grund.md#gnd-001-fissile-keep-files-small-on-every-commit-with-architecture-aware-overflow-messages) promised never lands. So speed is not a target; it is the *ordering principle* of this tool. When a design choice trades clarity, generality, or features for raw throughput on the hook path, raw throughput wins.

### 1. Performance targets

- Under **50 ms** on a typical commit (a handful of staged files on a developer laptop). The hook must be invisible relative to git's own cost.
- Under **500 ms** for a 10k-file batch through the library core. The whole-repo scan a CLI builds on top must be cheap enough to run from a watch loop or a CI smoke job, not just nightly.
- Single allocation per file at most; zero allocations on the hot size-check path where possible.

### 2. How we get there

- Linear pass per file, no second walks. Size in bytes is `stat`; size in lines is a streaming `memchr` for `\n`; both stop the moment the budget is exceeded.
- Skip files that obviously cannot violate the budget (size in bytes < smallest configured limit) before opening them.
- Parallel walk via `rayon` once the single-thread version stops winning on real repos.
- No regex on the hot path. Glob match for path → rule is compiled once and reused.

### 3. Measurable

Manual timing on the library core and on a synthetic 10k-file batch should stay within the targets above. The instruction-counting `cargo bench` harness over the hot paths ([§AR-002-instruction-benchmarks](architecture/AR-002-instruction-benchmarks.md#ar-002-instruction-benchmarks-instruction-counting-benchmarks-for-hot-library-paths), run in CI by [§AR-001-ci.5](architecture/AR-001-ci.md#5-benchmark-job)) records the per-commit number and fails pull requests that regress beyond the configured threshold. Alongside it, CI runs a release-mode 10k-file smoke test under a generous timeout ([§AR-001-ci.4](architecture/AR-001-ci.md#4-performance-smoke-guard)) so a catastrophic regression — an accidental quadratic path or a repeated pass over every file — fails the build outright.

## GOAL-002-tiny-footprint: one small static binary, no runtime

This tool is dropped into other people's repos. Every megabyte it adds and every runtime it requires is friction that the next contributor pays. So: a single statically-linked binary, on the order of a few MB stripped, with no Python/Node/JVM dependency at install time and a dependency tree small enough to audit on one screen. This is the *adoption* contract for [§GND-001-fissile](grund.md#gnd-001-fissile-keep-files-small-on-every-commit-with-architecture-aware-overflow-messages): a tool that nags you to keep your repo small had better keep itself small first.

### 1. Hard requirements

- One artifact per platform. Linux / macOS / Windows, each a single executable.
- No shared-library dependency beyond libc.
- No bundled tokenizer model files in the default build. Token-mode support, when on, links a small embedded tokenizer or shells out to a configured one ([§GOAL-005-configurable](goals.md#goal-005-configurable-every-limit-and-message-overridable-per-file-type-and-path)); the default binary stays small.
- Cold start under **10 ms** on a cached binary. Anything slower turns up in the [§GOAL-001-fast-feedback](goals.md#goal-001-fast-feedback-the-hook-is-imperceptible) budget.

### 2. What this rules out

Plugin systems, scripting languages embedded in the checker, "extensible" detector pipelines. Configuration via [§GOAL-005-configurable](goals.md#goal-005-configurable-every-limit-and-message-overridable-per-file-type-and-path) is data, not code; if a project needs custom logic, it can shell out to another binary from its own pre-commit config — not from inside this one.

### 3. Measurable

CI publishes the stripped binary size per platform on every release. A regression past a documented threshold fails the build. The dependency count in `Cargo.lock` (excluding stdlib and proc-macros) stays bounded and is reviewed at release time.

## GOAL-003-friendly-output: tell the user exactly what broke and how to fix it

A pre-commit hook that says "rejected" is worse than no hook at all. When the checker stops a commit, the contributor must know — without re-running anything, without reading docs — which file, which limit, by how much, and what local architectural move is expected next. This is the half of the contract a human reads. The other half ([§GOAL-004-token-thrift](goals.md#goal-004-token-thrift-the-tool-itself-spends-as-few-tokens-as-it-saves)) is what an agent reads; the two compose.

### 1. Hard requirements

- **Errors point at the file.** Every diagnostic is `path: <actual> <unit> exceeds <limit> <unit> (rule: <rule-name>)`. Editors and agents jump to the path unmodified.
- **The fix is named.** When a limit was crossed, the diagnostic names the config key that controls it (e.g. `[limits.ts] lines = 400`) and the message rule that produced the remediation text.
- **The local remedy is present.** An overflow includes the project's configured guidance for that rule: destination module, ownership boundary, extraction pattern, or architecture citation ([§GOAL-008-architecture-aware-messages](goals.md#goal-008-architecture-aware-messages-overflows-explain-the-local-architecture)).
- **Output is parseable.** A `--format=json` flag emits a stable JSON shape, one record per violation, suitable for LLM consumption and editor integration.
- **Help is one screen.** `fissile --help` fits in 24 lines and every flag carries a one-line example.
- **Explicit success.** A passing text run prints exactly `ok` on stdout; the JSON form stays diagnostics-only.

### 2. What this rules out

Configurable severity levels (would let two installs disagree on whether a repo passes), interactive prompts on the hook path (would block CI), and "did you mean" suggestions on config keys (a clean schema error is shorter and more honest).

### 3. Measurable

Every diagnostic emitted by the tool is matched by an e2e fixture asserting its exact finding shape and selected message ID. The `--help` screen length is enforced by a test. The JSON schema is published alongside the binary.

## GOAL-004-token-thrift: the tool itself spends as few tokens as it saves

The whole point of this checker is to lower the token cost of working in the repo. A checker whose own output is verbose, repetitive, or wraps each finding in generic preamble is at war with its own purpose. So: every byte the tool prints, an agent will eventually read — make each one carry weight. Custom messages are allowed because they replace rediscovery with local architectural guidance, but they stay bounded and structured.

### 1. What this requires

- **One compact record per finding.** A violation has one required finding line: `path: <actual> > <limit> [rule, message: <id>]`. Text output may add one configured guidance line; JSON carries the same message fields in the record.
- **No re-statement of inputs.** The tool does not echo back its configuration or the list of files it scanned unless `--verbose` is passed.
- **`--format=json` is the agent surface.** Agents are nudged toward JSON; the schema is a flat array of records, one per violation, with no envelope.
- **The audit summary is a count, not a paragraph.** `audit` ends in a single line: `<N> file(s) over limit`, or nothing if clean.
- **Stable byte output.** Same `(tree, config)` → same bytes. Findings are sorted deterministically (path, then rule name). An agent that diffs two runs sees only the real change.

### 2. What this rules out

Progress bars on the hook path (an agent piping the output gets a wall of escape sequences). Generic "helpful" copy ("Looks like you've added a large file!"), emoji status, and unbounded templates. Output that varies with terminal width on stdout (color/TTY affordances stay on stderr, opt-out via `--no-color`).

### 3. Measurable

On a 10k-file fixture with N violations, total stdout is under `N * 240` bytes plus a one-line trailer. An e2e fixture pipes the JSON output of `check` into `check` again (idempotence of the violation list) and diffs to zero.

## GOAL-005-configurable: every limit and message overridable per file type and path

Sensible defaults out of the box; full override when a project's reality diverges. A 2,000-line generated SQL dump is fine and a 2,000-line hand-written TypeScript module is not, and the tool only earns its keep when it can tell those apart and explain the local split that should happen. Configuration is data — a single TOML file at the repo root — not a plugin surface ([§GOAL-002-tiny-footprint.2](goals.md#2-what-this-rules-out)).

### 1. What is configurable

- **Limits per unit.** Bytes, lines, or tokens. A rule names the unit it uses; mixing is allowed across rules but not within one.
- **Limits per file type.** Per extension (`.ts`, `.py`, …) and per glob (`src/**/*.gen.rs`). Each rule carries both a soft and a hard limit ([§GOAL-006-graded-limits](goals.md#goal-006-graded-limits-soft-warns-hard-blocks-ai-minimizes)).
- **Exclusions.** Globs that opt files out of being checked at all — lockfiles, vendored code, generated artifacts, binaries. Exclusions need no rationale because the tool obviously does not apply; defaults cover the obvious cases so a fresh install does not immediately false-positive. Distinct from *exceptions* ([§GOAL-007-justified-exceptions](goals.md#goal-007-justified-exceptions-every-oversized-file-has-a-written-reason)), which keep the file under check but accept it as oversized for a written reason in the soft or hard registry.
- **Overflow messages.** Each rule may name a message template. Templates are short text blocks with stable IDs, inline citations when useful, and optional owner, destination path, and suggested split fields.
- **Scope.** Which directories are walked by `audit`; pre-commit always scopes to staged files.
- **Output defaults.** Format (`text` / `json`) and color, overridable per invocation.

### 2. What is NOT configurable

Per [§GOAL-003-friendly-output.2](goals.md#2-what-this-rules-out), the severity model, exit-code mapping, and machine-readable diagnostic fields are fixed. Two correctly-configured installs must agree on whether a repo passes. The human guidance text is intentionally project-owned.

### 3. Composition with the others

Configurability is the safety valve that lets [§GOAL-001-fast-feedback](goals.md#goal-001-fast-feedback-the-hook-is-imperceptible) hold its ground: the hot path stays fast because the user can exclude the file types that would otherwise force expensive checks (token counting on a vendored corpus, line counting on a minified bundle). It is also what keeps [§GOAL-002-tiny-footprint](goals.md#goal-002-tiny-footprint-one-small-static-binary-no-runtime) honest — a tokenizer is opt-in, not bundled by default.

### 4. Measurable

An e2e fixture with a non-default config (custom per-extension limits, a glob exclusion, a token-unit rule, and a custom overflow message) passes. The default config — applied implicitly when no config file is present — is checked into the e2e suite and snapshot-tested so a silent change to defaults fails CI.

## GOAL-006-graded-limits: soft warns, hard blocks, AI minimizes

A single threshold is a false economy. Set it low and the tool nags constantly until contributors disable it; set it high and the tool catches nothing until a file is already a problem. So this checker carries two limits per rule: a **soft** limit that warns, and a **hard** limit that fails. The contract is asymmetric on purpose — the warning is for the AI agent who can refactor cheaply mid-edit, the failure is for the human reviewer who can't be relied on to notice.

### 1. The two tiers

- **Soft limit.** A file at or above the soft limit emits a warning. The commit is not blocked. The diagnostic names the file, the size, the soft limit, and the rule. An AI agent reading the output is expected to attempt to reduce the file — split it, extract a helper, prune dead code — before claiming the task done. A human can ignore it; the friction is intentional but bounded.
- **Hard limit.** A file at or above the hard limit fails the commit. There is no override flag and no severity knob ([§GOAL-003-friendly-output.2](goals.md#2-what-this-rules-out)). The only way past it is the structured hard exception registry of [§GOAL-007-justified-exceptions](goals.md#goal-007-justified-exceptions-every-oversized-file-has-a-written-reason).
- **Order.** A file above the hard limit reports only the hard violation; the soft warning is implied. If a hard exception silences that hard violation, the soft warning may still appear unless the soft registry also accepts it.

### 2. The AI-minimize contract

The warning is the load-bearing surface for an agent. Its finding shape is fixed so an agent can pattern-match on it without prose parsing: `path: <actual> <unit> > <soft-limit> <unit> [soft, rule: <name>, message: <id>]`. The companion line in the managed `AGENTS.md` block (written by `fissile init`, parallel to grund's pattern) names this output and the expected response: *if you wrote this file in this turn, follow the configured architecture guidance and try to bring it back under the soft limit; if you did not, leave it alone unless the task is about that file*.

This is the half of [§GND-001-fissile](grund.md#gnd-001-fissile-keep-files-small-on-every-commit-with-architecture-aware-overflow-messages)'s promise that pays off continuously — not by blocking commits, but by making "shrink the file you just grew in the way this repo expects" the path of least resistance for the agent that grew it.

### 3. What this rules out

- **A single configurable threshold.** Two installs of the tool that share a config must agree on whether a file warns, fails, or passes. The soft/hard pair is the schema; you cannot collapse them into one or expand them into three.
- **A `--strict` flag that turns warnings into errors.** That would let CI and local hook disagree, which is exactly the surprise [§GOAL-003-friendly-output](goals.md#goal-003-friendly-output-tell-the-user-exactly-what-broke-and-how-to-fix-it) refuses.
- **Unstructured warning text.** The finding is byte-stable and the configured message has a stable ID so agents can pattern-match on it ([§GOAL-004-token-thrift.1](goals.md#1-what-this-requires)).

### 4. Measurable

E2E fixtures cover all four states per rule: clean, soft-only, hard-only, both. The hard-only case must exit non-zero; the soft-only case must exit zero with the warning on stderr. A fixture that wires the soft warning through a mock agent loop asserts the diagnostic shape an agent would key off.

## GOAL-007-justified-exceptions: every oversized file has a written reason

The hard limit ([§GOAL-006-graded-limits](goals.md#goal-006-graded-limits-soft-warns-hard-blocks-ai-minimizes)) has no override flag. It does have one escape hatch — a structured hard exceptions registry where each oversized file is declared with a written rationale and a maximum accepted measurement. Soft warnings have a parallel soft exceptions registry for agent-facing debt that the repository has deliberately accepted. The registries are the paper trail: every accepted oversized file has, somewhere in the repo, a paragraph explaining why, and that paragraph carries a stable local ID for output and review.

This is the contract that lets [§GND-001-fissile](grund.md#gnd-001-fissile-keep-files-small-on-every-commit-with-architecture-aware-overflow-messages) hold under real-world adoption. A team turning the tool on for the first time will have files over the soft and hard limits on day one; the registries are how they accept the current state without disabling the guard, and how the next reviewer or agent can tell "this file is large for a reason" from "this file is large because nobody noticed."

### 1. What the registries are

Two TOML files at configured paths in the target repo hold one structured entry per exempted file. The soft registry defaults to `docs/file-size-agent-exceptions.toml`; the hard registry defaults to `docs/file-size-human-exceptions.toml`. Each entry looks like:

```toml
fissile_exceptions_version = 1

[[exceptions]]
id = "EX-NNN-slug"
path = "path/to/file.ts"
match = "exact"
rules = ["rule-id"]
max_accepted = { value = 800, unit = "lines" }
until = "condition, date, or indefinite"
reason = """
One paragraph of rationale: why this file is large, what would need to change
for the exception to be retired, and who to ask before deleting it.
"""
```

The `EX-` ID is local to `fissile`; the parsing contract lives in
§FS-003-exceptions.

### 2. How the checker uses it

- On startup the checker loads both registries, parses each `[[exceptions]]` entry, extracts the path or glob from the structured fields, and builds a path → exception index per severity.
- A file matched by a soft-registry exception is silenced for soft findings at or below the entry's `max_accepted` value. A file matched by a hard-registry exception is silenced for hard findings at or below the entry's `max_accepted` value.
- If the file grows past `max_accepted`, the finding appears again even though the path still has an exception entry.
- `fissile exception add` (§FS-005-exception-add) is the supported way to append
  entries, so users do not need to hand-edit registry TOML for current
  overflows.
- An exception whose path matches no file under scan is reported by `audit --stale` — dead exceptions rot fast and the tool refuses to pretend they are load-bearing.
- The diagnostic for a file under exception still names the exception ID on `audit --verbose`, so reviewers can find the rationale without grep.

### 3. What this rules out

- **An exception without a rationale.** The schema requires the prose paragraph; an entry with empty body is a parse error. Silent override is exactly what [§GOAL-003-friendly-output](goals.md#goal-003-friendly-output-tell-the-user-exactly-what-broke-and-how-to-fix-it) refuses.
- **A flag-based override.** No `--allow path/to/file`, no `# fissile: allow` magic comment. The registries are the only escape hatch, because they are the only form that survives review and shows up in history.
- **An exception that lives next to its file.** Centralizing the registries is what makes the inventory legible: one file lists soft agent debt and one file lists hard human debt. A scattered "exemption per file" cannot answer "show me everything we have given up on."

### 4. Composition with the other goals

- [§GOAL-006-graded-limits](goals.md#goal-006-graded-limits-soft-warns-hard-blocks-ai-minimizes) defines what the registries override; this goal defines the shape of the override.
- [§GOAL-005-configurable.1](goals.md#1-what-is-configurable) separates exclusions (no rationale, for files the tool obviously does not apply to) from exceptions (rationale required, for files the tool applies to but accepts). The split is deliberate — silencing a `.png` and silencing a 4,000-line module are not the same kind of decision and the registry shapes should not let them look the same.
- [§GOAL-004-token-thrift](goals.md#goal-004-token-thrift-the-tool-itself-spends-as-few-tokens-as-it-saves) holds: a file under exception emits no diagnostic by default, only on `--verbose`.

### 5. Measurable

E2E fixtures cover: `fissile exception add` appending soft and hard entries; a hard registry with a single exception silencing a hard violation; a soft registry with a single exception silencing a soft warning; a file that outgrows its exception's maximum accepted size and reports again; a registry whose exception names a path that no longer exists (audit must flag it stale); and a registry entry whose rationale is empty (parse error). A snapshot test on the default registry paths and the parse rules guards against silent schema changes.

## GOAL-008-architecture-aware-messages: overflows explain the local architecture

The key promise of [§GND-001-fissile](grund.md#gnd-001-fissile-keep-files-small-on-every-commit-with-architecture-aware-overflow-messages) is not only that the library notices large files on every commit. It tells the contributor what the repository already knows: which architectural boundary the file is pressing against, where new code should move, and which local rule explains that move.

### 1. What the message knows

- **The matched rule.** The message is selected by the same rule that selected the budget, so `src/http/**` and `src/domain/**` can teach different split patterns.
- **The architecture citation.** A message's text may cite a `§AR-`, `§FS-`, or `§GOAL-` ID so the reader can pull deeper context only when needed.
- **The destination hint.** A message may name a module, directory, owner, or interface boundary that should receive the extracted code.
- **The expected action.** The text says what to do next: split a helper, extract a fixture, move generated output, add an exception, or tighten the rule.

### 2. Constraints

Messages are project-owned but not arbitrary scripts. They are static templates with bounded length, stable IDs, and explicit owner/destination/action fields. They cannot run code, inspect file contents beyond the matched rule, or change pass/fail behavior. The finding remains machine-readable; the message is the local architectural explanation layered on top.

### 3. Measurable

E2E fixtures cover rule-specific messages, a message that cites an architecture declaration, a missing citation that fails validation, and JSON output that includes both the message ID and rendered guidance.
