# PROTOTYPES — throwaway, do not carry forward

Built to answer [#8 Prototype: pick the client stack](https://github.com/amin-bf/leitner/issues/8).
Delete once the ADR lands. Nothing here is production code: no tests, no error handling beyond what
makes it run, no abstractions worth keeping.

## The slice, identical in both

Small but load-bearing, per the ticket:

1. Show a card front (3 hardcoded cards, no scheduling).
2. Accept a graded answer — 4 grades, `1 Forgot / 2 Barely / 3 Good / 4 Easy`, per ADR-0001.
3. Show the back.
4. Append the review event to a **local append-only log**.
5. **Survive a restart** — on launch, read the log back and show the event count and the last few.

The event, byte-identical in both stacks:

```json
{"card_id":0,"grade":3,"at_ms":1753600000000,"device":"<stack>-<platform>"}
```

## The storage seam

Both slices put the seam at **bytes**, not SQL — the "low seam" of
[`docs/research/client-stacks/README.md`](../docs/research/client-stacks/README.md) §3.4, because the
map's data model is an append-only event log and a byte seam is the only option that keeps
redb/fjall on the table.

One `Store` type per target family, same call sites:

| Target | Backend |
|---|---|
| Desktop | append to a JSONL file under the user data dir |
| Android | append to a JSONL file under `Context.getFilesDir()` |
| Web | append to a JSONL file in **OPFS** |

## The two slices

| Dir | Stack |
|---|---|
| [`dioxus-slice/`](./dioxus-slice) | Dioxus 0.7.9 — one crate, `dx serve --platform {desktop,web,android}` |
| [`tauri-leptos-slice/`](./tauri-leptos-slice) | Leptos 0.8 CSR + Tauri 2 — frontend crate + core crate + shared crate |

Each has a `README.md` (what it is, how to run it, what was verified) and a **`DEV-NOTES.md`**.

## Which file survives the decision

The prototypes are throwaway; the knowledge is not.

- [`COMPARISON.md`](./COMPARISON.md) — the measured evidence. Feeds the **ADR** for #8, then can go.
- `<winner>/DEV-NOTES.md` — prerequisites, commands, storage locations, every trap hit, and the
  rules an agent needs. **This is the source for the repo `README.md` (setup + commands) and
  `AGENTS.md` (traps + working rules)** once a stack is chosen.
- The loser's `DEV-NOTES.md`, both slices, and this directory — delete.
