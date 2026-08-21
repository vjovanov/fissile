# DF-007-instructions-at-the-error-site: An instruction lives where the decision is made, not in the always-loaded block.

`fissile init` installs a managed block into every agent entrypoint a repository
keeps (§FS-002-init.3). That block is context an agent loads at the start of
every session, whether or not it ever touches an oversized file. Through v2 it
carried nine rules — the whole workflow, from `check --staged` to `audit
--stale-exceptions` — at a cost paid by every session in the repository.

Most of those rules have a moment they apply, and at that moment `fissile` is
already speaking. `--kind` and `--until` were never in the block and are not
missed: the refusal explains them where the entry is written (§FS-005-exception-add.1).

## 1. Decision

An instruction belongs in an error surface when it has one. It belongs in the
managed block only when it has to be known *before* the tool speaks.

- **The error surface is the default.** Anything a `check` finding, an
  `exception add` refusal, or an `audit` section can say at the point of the
  decision is said there and left out of the block.
- **The block keeps what arrives too late otherwise.** Three things survive:
  that this repository caps file size at all, that `fissile check --staged` is
  run before calling work done, and that the gate is not to be bypassed with
  `--no-verify`. The first changes where code lands while it is being written,
  which no finding can do — a finding arrives after the file is already long.
  The second and third reach the agent in repositories where no hook fires
  (§FS-002-init.6 installs into `.git/hooks/pre-commit` only) and reach the
  agent that is looking for a way past a hook that just blocked it, which is
  the least receptive possible audience for the hook's own text.
- **Duplication is a cost, not a safety net.** A rule the tool already prints at
  the right moment is not repeated in the block for emphasis. The v2 block
  restated the soft/hard model, the seam rule, and the exception route, all of
  which the shipped `[[messages]]` already say (§DF-003-severity-guidance.1).

## 2. What this costs

§GOAL-004-token-thrift rules out generic helpful copy in output, and this
decision adds two lines that are not findings: a `hint:` line naming
`fissile measure` when a run reports anything, and a commit-gate epilogue under
`check --staged` when a hard finding stands (§FS-004-check-audit.1).

Both are bounded — one line each, once per run, never per file — and both are
paid only by a run that already failed. They replace six lines of block text
that every session paid for unconditionally. A repository whose commits are
mostly clean now spends nothing on the guidance it does not need, which is the
same goal read forward instead of backward.

## 3. Rejected alternatives

- **Empty the block entirely and let the hook teach everything.** The hook fires
  after the code is written, so the file is already too long by the time
  anything is said; a repository that routes hooks elsewhere gets no signal at
  all; and `--no-verify` is argued against only by the thing being bypassed.
- **Put the surviving text in the shipped `[[messages]]` instead.** That text is
  written to be replaced — the generated config tells a project to rewrite it
  with its own destinations (§DF-003-severity-guidance.1) — so anything embedded
  there is gone in the first repository that tunes its guidance.
- **Keep v2 and shorten the wording.** The block's cost is structural, not
  stylistic: nine rules stay nine rules however tightly they are phrased, and
  eight of them are read by a session that never overflows a file.

## 4. Consequences

The managed block goes from thirty-five lines to five, and the version moves
into begin/end markers so a shortened block can be recognized and replaced
without depending on what its heading says (§FS-002-init.4).

New guidance now needs a home before it can be written: an instruction with no
error surface either gets one or is not shipped. That is a constraint on the
tool, and the intended one — a rule nobody can be told at the right moment is a
rule that was going to be read too late anyway.
