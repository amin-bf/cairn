# ADR-0027: The scheduler crate is `leitner-core`'s one dependency

- **Status**: Accepted
- **Date**: 2026-08-01
- **Surfaced by**: [The domain spine: content, scheduling, the log form, and replay](https://github.com/amin-bf/cairn/issues/78)
- **Related**: [ADR-0001 §1 §6 §7](0001-scheduling-algorithm-and-grade-scale.md) (FSRS-6 via the
  crate, pinned exactly; the parameter vector; fuzz seeded from card identity),
  [ADR-0009 §2 §3 §6 §8](0009-crate-and-workspace-layout.md) (the empty dependency list, contexts as
  modules, the `log` naming hazard, values rather than injected traits — this ADR **amends §2**),
  [ADR-0007 §3](0007-the-local-store.md) (the derivation version carries the pinned crate version),
  [ADR-0014 §3](0014-when-parameter-optimisation-runs.md) (the optimiser on a worker thread)

**This is the first ADR written after [the map](https://github.com/amin-bf/cairn/issues/1) closed**,
and it is the shape the map's own closing note predicted: not fog, but a decision that fell between
two accepted documents because each believed the other owned it. It was found by writing the first
implementation ticket, which is the earliest anything could have found it.

## Context

Three accepted statements cannot all hold.

- [ADR-0001 §1](0001-scheduling-algorithm-and-grade-scale.md) makes the scheduler **FSRS-6 via the
  `fsrs` crate, pinned exactly**, and rejects both writing SM-2 ourselves and inventing a graded
  Leitner engine — *"it would require inventing the core of the product's correctness with no
  evidence behind it."*
- [ADR-0009 §3](0009-crate-and-workspace-layout.md) places `scheduling` **inside `leitner-core`**, as
  a module rather than a crate, because nothing varies across a context line.
- [ADR-0009 §2](0009-crate-and-workspace-layout.md) says that crate's *"`[dependencies]` section is
  empty, deliberately and permanently. No `rusqlite`, no `egui`, no clock, no random number
  generator, no serialisation crate."* `crates/core/Cargo.toml` repeats it in a comment.

No ADR reconciles them. `fsrs` appears in ADR-0001, ADR-0004, ADR-0007 and ADR-0011, and in none of
them is it a question about crate placement — so the collision was never in anyone's field of view.

**The dependency is not one crate.** `fsrs` 6.6.1 declares nine mandatory dependencies —
`itertools`, `log`, `ndarray`, `priority-queue`, `rand`, `rayon`, `serde`, `snafu` and `strum` — and
its `default` feature set is empty with only `experimental_cost_adr` beside it. **There is no feature
that separates the scheduler from the optimiser**, and none that trims the tree. So the question is
not whether to admit a small pure crate; it is whether to admit that tree, which contains an RNG and
a serialisation framework — two of the four things §2 names by name.

## Decision

### 1. `fsrs` is a dependency of `leitner-core`, and it is the only one

`crates/core/Cargo.toml` gains exactly one entry, pinned exactly per
[ADR-0001 §1](0001-scheduling-algorithm-and-grade-scale.md) — the 6.x line shipped nine releases in
eleven days with breaking API changes inside the major version, so the pin is not caution but a
requirement. Where the version is declared follows the workspace's existing convention for keeping
crates from drifting apart.

**The property [ADR-0009 §2](0009-crate-and-workspace-layout.md) was protecting survives untouched**:
`cargo test -p leitner-core` still needs **no database, no window and no handset**. That is the
sentence §2 wrote down as the prize, and it is behavioural — it says where the tests can run and what
the domain can reach. Nothing in the `fsrs` tree opens a file, draws a pixel, reads a clock or talks
to a device.

What does not survive is §2's **letter**: the section is no longer empty, and `rand` and `serde`
become reachable transitively. That is a real cost and it is stated here rather than glossed, because
§2's force came from being absolute.

### 2. The admission test, so this is a rule and not an exception

§2 already says a dependency here *"is a decision, not a convenience"*. It did not say what would
qualify, because under a permanently-empty section nothing could. Now that one entry exists, the test
has to be written down or the next one arrives by analogy.

A crate may be added to `leitner-core` only if **all** of these hold:

1. **The specification mandates it by name.** Not "we need a hashmap" — an accepted ADR names this
   crate, or names an algorithm only this crate implements. `fsrs` qualifies through ADR-0001 §1.
2. **It performs computation and nothing else.** No I/O, no clock, no platform, no network, no
   process. The test is §2's own: can `cargo test -p leitner-core` still run with no database, no
   window and no handset?
3. **It builds for every supported target.** Desktop and Android today. Established for `fsrs` by
   [#20](https://github.com/amin-bf/cairn/issues/20), which linked and ran it on the handset.
4. **It is pinned exactly**, for the reason ADR-0001 §1 gives.
5. **A new ADR records it.** This one is the precedent for the form, not for the leniency.

Failing any of the five means the answer is no, and the argument to have is about where the code
belongs rather than about whether the rule can bend.

### 3. What arrives transitively is not precedent for what may arrive directly

`rand`, `serde`, `rayon` and `ndarray` are now compiled into the domain crate. **None of them is
thereby available to our code**, and none of them may be reached for.

This matters most for the two §2 names among them. **`rand` is not a licence to randomise**:
[ADR-0001 §7](0001-scheduling-algorithm-and-grade-scale.md) seeds fuzz from `CardRef` precisely so
that two devices replaying one log agree, and an RNG in replay voids the entire merge design.
**`serde` is not a licence to derive serialisation**: [ADR-0004 §11](0004-the-review-event-log.md)
relays the interchange line **byte for byte and never re-encodes it**, which is a stronger guarantee
than any derive can offer and is the reason [ADR-0007 §2](0007-the-local-store.md) could make the
stored line authoritative.

A future reader finding `rand` in the lockfile and concluding the prohibition lapsed would be making
exactly the mistake this section exists to pre-empt.

### 4. The `log` naming hazard is untouched, and the reason is worth stating

[ADR-0009 §6](0009-crate-and-workspace-layout.md) and `log`'s own `CONTEXT.md` record that the module
called `log` would shadow the `log` crate, and treat the empty dependency list as what makes the
collision *unreachable rather than merely unlikely*.

`fsrs` depends on `log`. **The hazard still cannot fire**, because only *direct* dependencies enter a
crate's extern prelude — a transitive one is invisible to our source. So §6's reasoning needs
restating rather than revisiting: the collision is now prevented by `log` not being a **direct**
dependency, which is a narrower guarantee than "no dependencies at all" and worth naming before
someone adds `log` for tracing and discovers it the hard way.

Logging still belongs at the edges, in `leitner-store` and `leitner-app`.

### 5. Two constraints travel with the crate, and neither is optional

**The fuzz is ours, not the crate's.** ADR-0001 §7 keeps interval fuzz but seeds it from the
`CardRef` encoding. `fsrs` ships its own fuzz over `rand`. The implementation therefore takes the
**un-fuzzed** interval and applies our own — and if the crate's API exposes only a fuzzed one, that
is a problem to solve in the open rather than a detail to settle quietly, because a fuzz the crate
seeds is a fuzz two devices do not agree on.

**`rayon` is compiled in and never reached.** [#20](https://github.com/amin-bf/cairn/issues/20)
confirmed by measurement what [#2](https://github.com/amin-bf/cairn/issues/2) inferred statically:
`compute_parameters` is single-threaded at runtime. Recorded so that nobody later reads the
dependency as an invitation to parallelise, and so that a future version that *does* reach it is
noticed as a change rather than absorbed as normal.

### 6. Why not a separate crate, and why not the values move

**A `leitner-scheduling` crate was the obvious alternative and does not stop at one crate.** `replay`
depends on `scheduling`, so either `replay` moves out of `leitner-core` too, or `leitner-core` takes
a workspace dependency and §2's "empty" breaks anyway — by a sibling instead of by `fsrs`, which buys
the letter of the rule and none of its meaning. Done honestly it splits the domain into content+log
here and scheduling+replay there, which is precisely the fragmentation
[ADR-0009 §3](0009-crate-and-workspace-layout.md) called *"the tell that the line was drawn in the
wrong place"*. It pays in architecture to buy an empty TOML section.

**Applying [ADR-0009 §8](0009-crate-and-workspace-layout.md)'s move was the better rejected option.**
§8 makes time and identity *values, never injected traits*; the same shape would have `replay` keep
ordering, cutoffs, `config-set` application and dormancy — all pure — and emit the ordered per-card
event stream, with the FSRS fold one crate out. `leitner-core` would stay literally empty, and the
[highest-value test in the repository](../../crates/core/src/replay/CONTEXT.md) would stay with it,
since merge commutativity is a property of **row ordering** rather than of FSRS output.

It is rejected because it splits what `replay`'s own `CONTEXT.md` calls **the join**. Memory state,
the box and the due day would leave the crate that exists to compute them, and `replay` — the one
context with no ADR of its own, whose whole justification is that its rules are incoherent when
scattered — would be scattered again across a crate boundary. The cure reproduces the disease.

**Reimplementing FSRS-6 was not seriously available.** ADR-0001 §1 already refused it, and there is a
sharper reason: the optimiser fits weights to *the crate's* formulas. A reimplementation that
diverges by a rounding rule produces parameters that are wrong for the arithmetic consuming them, and
the disagreement is invisible — every value still looks like a plausible interval.

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [0009 §2](0009-crate-and-workspace-layout.md) | *"`[dependencies]` … empty, deliberately and permanently"* becomes **one entry, admitted by §2 above's five-part test**. The prohibition list stands as written for anything **direct**; `rand` and `serde` are now present **transitively** and remain unreachable by our code. | §1 and §3 above. §2's stated prize — no database, no window, no handset — is behavioural and survives intact; its absolute phrasing did not survive contact with ADR-0001 §1, which named a crate the domain cannot do without. |
| [0009 §6](0009-crate-and-workspace-layout.md) | The `log` collision is prevented by `log` not being a **direct** dependency, rather than by there being no dependencies at all. | §4 above. The guarantee narrowed; the hazard did not change. Someone adding `log` for tracing needs to meet the rule, not discover it. |

## Consequences

- **`cargo test -p leitner-core` keeps its promise**, which is the one thing this decision was
  measured against. Most of the specification stays verifiable with no database, no window and no
  handset, and that remains build-checked rather than intended.
- **The domain crate's dependency list is now a surface that has to be defended**, where before it
  was defended by being empty. §2's five-part test is the whole defence, and a review that waves a
  sixth entry through on "we already have one" is the failure mode.
- **`leitner-core` now compiles a threadpool, an RNG, a serialisation framework and an n-d array
  library.** Build times rise and the crate stops being inspectable at a glance. This is the price,
  and it was paid for correctness of the scheduler rather than for convenience.
- **The derivation version's dependence on the pinned crate version
  ([ADR-0007 §3](0007-the-local-store.md)) is now a dependency between two files that must move
  together.** Bumping `fsrs` invalidates cached scheduling state by design; the bump and the
  derivation-version change are one commit, never two.
- **A version bump of `fsrs` is not routine.** It moves the arithmetic, the optimiser and the
  derivation version at once, and ADR-0001 §1's exact pin exists because that line has broken its own
  API inside a major version.

## Open items handed onward

- **Whether the crate exposes an un-fuzzed interval** (§5) — [#78](https://github.com/amin-bf/cairn/issues/78)'s,
  and the answer is a fact to be read out of the pinned version rather than a decision.
- **Nothing else.** The contradiction is resolved, the rule that replaces it is stated, and what the
  dependency does *not* license is written down.
