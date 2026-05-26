# DF-001-tool-name: Name the tool `fissile`.

The project ships as a single CLI binary and needs a name that is clean to publish,
that a human and an agent both read correctly on first contact, and that reinforces
what the tool does rather than fighting it. After surveying physics and biology
metaphors for "a thing splits once it grows too big," the name is **`fissile`** — the
crate, the binary, and the command. The working title `small-files` is retired to a
description, not the name.

## 1. Decision

The tool is named `fissile`. The published crate is `fissile`, the installed binary is
`fissile`, and the primary invocations read `fissile check` / `fissile audit` /
`fissile init`.

## 2. Why `fissile`

`fissile` is the adjective for matter past the threshold at which it undergoes
*spontaneous fission* — it splits under its own accumulated mass, with no external
trigger. That is exactly the condition this tool reports: a file that has crossed a
size budget will be split, deliberately now or messily later.

The decisive property is that `fissile` names a **state**, not an action. Every other
candidate (`fission`, `calve`, `mitosis`) names the verb "split"; `fissile` names the
property "ready to split, under its own weight." That is precisely what the soft-limit
diagnostic asserts about a file (§GOAL-006-graded-limits), so the tool's name and its
load-bearing output describe the same thing. An agent reading `file X is fissile: 612
lines > 400` learns the *why*, not merely the *what* (§GOAL-003-friendly-output).

Secondary support:

- **First impression is on-metaphor.** Cold readers map `fissile` to "nuclear /
  splitting" with no explanation — unlike `calve` (reads as cattle first, glacier
  fourth) or `ripple` (reads as the cryptocurrency).
- **Clean to publish.** `fissile` is unregistered on crates.io as of 2026-05-20; the
  only npm collision is an abandoned 2021 frontend project, irrelevant to a Rust CLI.
- **Cheap to type and tokenize.** `fissile` is two tokens; `fissile check` is shorter
  than `small-files check` in both bytes and tokens (§GOAL-004-token-thrift).

## 3. Alternatives rejected

- **`calve`** — clean namespace and a vivid glacier-calving image, but the first
  impression in English is *baby cows*, with the iceberg sense a distant fourth. A name
  whose intended metaphor is the fourth association fights itself. Held as the fallback
  if `fissile` becomes unavailable.
- **`fission`** — punchy and on-metaphor, but it names the action rather than the state,
  carries a violent tone at odds with the "AI minimizes" framing
  (§GOAL-006-graded-limits), and a 25-day-old squat already sits at `fission` 0.1.0 on
  crates.io.
- **`mitosis`** — strong biological metaphor, but an active crates.io crate since 2019
  and Builder.io's well-known `mitosis` compiler poison both the namespace and search.
- **`planck`** — taken on crates.io by another lightweight-utility namespace and
  dominated by planck.js (physics engine) on npm; the first impression is unrelated to
  file size.
- **`ripple`** — names the *problem* (cost rippling outward through every agent run),
  not the cure, and the search term is owned by the XRP cryptocurrency.
- **`scission` / `spallation`** — both clean on both registries but rejected on
  readability: `scission` reads clinical, `spallation` is long and obscure. Kept as
  clean fallbacks behind `calve`.

## 4. Consequences

- Crate, binary, and command are all `fissile`; the repository may be `fissile` or
  `fissile-rs`.
- The soft-limit diagnostic vocabulary can lean on the name — a warned file *is*
  "fissile" — which is self-documenting against §GOAL-006-graded-limits.
- Re-check crates.io availability immediately before first publish: the name was free on
  2026-05-20 but is unregistered, so it is first-come.
