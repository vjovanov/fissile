# AR-001-ci: CI mirrors the local quality gate and records performance

The CI workflow is the remote form of the local development gate. Anything that
can make the crate unbuildable, untestable, unformatted, ungrounded, or too slow
must fail in CI, so contributors cannot bypass the checks by skipping local hooks
or editing through a web UI. This supports §GOAL-001-fast-feedback and
§GOAL-003-friendly-output.

## 1. Matrix

The Rust build and test matrix runs on Linux, macOS, and Windows. Each leg
installs stable Rust with `rustfmt` and `clippy`, restores Cargo caches on a
best-effort basis, checks formatting, builds all targets with warnings denied,
runs all tests, and runs clippy with warnings denied.

Cache failures are not build failures. A cold cache must still reach the actual
format/build/test/lint steps and let those steps decide pass or fail.

## 2. Grounding

The Linux leg runs `grund check` so docs, source citations, and architecture
references stay valid. This is separate from Rust compilation: `cargo` proves the
crate builds, while `grund` proves the project explanation still resolves.

## 3. Packaging

The Linux leg runs `cargo package --locked --list` as a cheap packaging sanity
check. It verifies that Cargo can assemble the crate contents under the locked
dependency graph without doing a publish.

## 4. Performance smoke guard

CI carries a cheap wall-clock backstop for §GOAL-001-fast-feedback: the matrix
runs the `large_batch_smoke` release test under a generous 30 second timeout. The
budget itself is much tighter than that; this guard is for catastrophic
regressions such as an accidental quadratic path or a repeated scan over every
file. The precise per-commit meter is the benchmark job in §5.

The test binary is compiled in a separate step before the timed one, so the
clock measures the scan and not the compiler: the release profile is LTO with a
single codegen unit, and a link that grows with the dependency tree would
otherwise register as a runtime regression it is not.

## 5. Benchmark job

A separate Linux-only `bench` job runs the instruction-counting harness
(§AR-002-instruction-benchmarks). The job installs Valgrind and the
`iai-callgrind-runner` version that matches the crate dependency. On pull
requests it first records a base-branch baseline, then reruns the pull request
with `--callgrind-limits=ir=5.0%`; instruction-count growth beyond that limit is
a build failure. Pushes to `main` record current counts and upload the JSON
summaries for inspection.

The benchmark body is gated behind the `bench` Cargo feature, so ordinary build
and test jobs compile a no-op bench target and never require Valgrind.

## 6. PGO pre-release check

PGO stays out of push and pull-request CI. The manual `Pre-release checks`
workflow installs `llvm-tools-preview` and runs `scripts/pgo-build.sh`. That
keeps the ordinary feedback loop focused on format, build, test, lint,
grounding, smoke, and instruction counts, while still proving the PGO toolchain
before a release.

The PGO script trains on two instrumented workloads before merging one profile:
the release test suite, and the `fissile` CLI hot commands (`check` and `audit`)
run over this repository. Training the real commit-time path keeps the profile
aligned with §GOAL-001-fast-feedback rather than with test scaffolding. The
merged profile then drives a final profile-use rebuild of the release artifacts
under `target/release`.

## 7. Binary-size guard

The pre-release workflow strips the release binary and fails if it exceeds a
documented ceiling, closing the loop on the footprint promise
(§GOAL-002-tiny-footprint.3). The ceiling is generous relative to the current
artifact; it exists to catch a dependency or feature that silently inflates the
single-binary contract, not to police small movements.

## 8. Release workflow

Releases are a workflow, not a checklist. `release.yml` runs on a `v*.*.*` tag
push or a manual dispatch that names the version, and it publishes everything a
consumer can install (§GOAL-002-tiny-footprint.1):

- **Verify.** The requested version must match `Cargo.toml` exactly, and when
  crates.io publishing is requested the registry token must be present before
  any long build starts.
- **Build.** PGO release binaries (§6) for six targets: `x86_64` and `aarch64`
  Linux built inside pinned manylinux2014 containers so the glibc baseline
  cannot drift, plus native macOS (Intel and Apple silicon) and Windows
  (`x86_64` required, `aarch64` with an LTO fallback when PGO training fails on
  hosted runners). Every binary must answer `fissile --version` with the version
  being released before it is packaged (§FS-006-cli.3), and every artifact is
  measured against the §7 size ceiling.
- **Publish.** `cargo publish` runs only after all binaries built, and skips
  silently when the exact `fissile@<version>` already exists so a re-run after a
  partial failure is safe. The GitHub release uploads one archive plus a
  `.sha256` per target and takes its notes verbatim from the released section of
  `docs/changelog.md` via `scripts/prepare_changelog_release.py`.

Two helper workflows prepare versions but never publish by themselves:
`auto-bump.yml` (scheduled) proposes a patch bump when substantive commits have
landed since the last tag and CI on the tip is green, and `release-minor.yml`
(manual) does the same for a minor bump. Both update `Cargo.toml`, the
version-pinned e2e case (§FS-006-cli.3), and the changelog via the same script,
then dispatch `release.yml`.

### 8.1 What the automation needs

Releases are meant to run without a human in the loop, so the standing state a
release depends on is recorded here rather than in someone's memory:

- **`CARGO_REGISTRY_TOKEN`** — a crates.io API token scoped to publish-update
  (and publish-new for the first release). `release.yml` fails fast when it is
  missing rather than after the build matrix.
- **`RELEASE_PAT`** — a fine-grained GitHub token with *Contents: read+write*
  and *Actions: read+write* on this repository. The bump workflows push the
  version commit straight to `main` and dispatch `release.yml`; the default
  `GITHUB_TOKEN` can do neither, because `main` requires pull requests and a
  `GITHUB_TOKEN` push never triggers another workflow. It expires — a silently
  failing Monday bump is the symptom.
- **Repository rulesets** — `main protection` (pull requests, linear history,
  the three `cargo test` matrix jobs as required checks) and `release tags`
  (`v*.*.*` cannot be deleted or force-moved). Both list the repository-admin
  role as a bypass actor, which is what lets the `RELEASE_PAT` push land.

The scheduled bump derives the next version from the latest `v*.*.*` tag, so a
repository with no tag yet cannot bootstrap itself: the first release is cut by
pushing `v<version>` (or dispatching `release.yml` with the version). Every
release after that is automatic.

### 8.2 Between releases, main carries a dev version

A release leaves main holding the version it just published, so every build from
main until the next release reports the tag it is already ahead of. Nothing then
distinguishes a binary built from main from the released one, and a fix that is
merged but not installed looks exactly like one that is installed.

The release therefore advances main as its last act: after publishing `X.Y.Z` it
commits `X.Y.(Z+1)-dev`. The suffix is what makes `fissile --version` say which
side of the tag a build came from. A `-dev` manifest is never publishable — the
release path sets and verifies the clean version on its candidate branch — so
the guarantee that a released version has a tag is unchanged.
