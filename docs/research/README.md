# Research findings

Point-in-time findings gathered while charting [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1). **These are not the spec.** The spec is the accumulated set of ADRs and per-context `CONTEXT.md` files described in [`docs/agents/domain.md`](../agents/domain.md); these documents are the evidence those decisions were made against.

Each report was produced by a `/research` subagent resolving one `wayfinder:research` ticket, and is dated by the ticket that commissioned it. Treat the facts as true as of that date and re-check anything load-bearing before relying on it — upstream crate health, maintenance status, and platform floors all drift.

| Report | Ticket | Answers |
| --- | --- | --- |
| [`scheduling-algorithms/`](./scheduling-algorithms/README.md) | [#2](https://github.com/amin-bf/leitner/issues/2) | SM-2 vs FSRS vs graded Leitner; cold-start behaviour, the box-metaphor failure mode, and the `fsrs` crate's licence and platform reach |
| [`client-stacks/`](./client-stacks/README.md) | [#3](https://github.com/amin-bf/leitner/issues/3) | Dioxus vs Leptos + Tauri for desktop/web/Android, and the cross-platform storage layer underneath both |
| [`local-first-event-log/`](./local-first-event-log/README.md) | [#4](https://github.com/amin-bf/leitner/issues/4) | Append-only review logs, cross-device merge, and how a device tells whether it is ahead of or behind another |
| [`sync-transport/`](./sync-transport/README.md) | [#33](https://github.com/amin-bf/leitner/issues/33) | What can carry the log between devices with no server of our own — a git remote, a rented object store, rented WebDAV, a personal cloud drive, or a folder another application syncs; whether conditional writes are needed at all, what segment granularity costs, and what Android permits unattended |
| [`auto-backup-at-rest/`](./auto-backup-at-rest/README.md) | [#60](https://github.com/amin-bf/leitner/issues/60) | Whether the platform's automatic app-data backup is readable by the company that stores it — the two encryption layers and which one is conditional, what a missing lock screen costs, where the guarantee starts relative to this project's API 24 floor, how the 25 MB quota fails, and how coarse the user's deletion control is |
| [`auto-backup-quota/`](./auto-backup-quota/README.md) | [#64](https://github.com/amin-bf/leitner/issues/64) | Whether the 25 MB automatic-backup quota is measured before or after compression — and therefore whether ADR-0016 §7's user-facing size sentence is true; also what unit the quota actually counts, whether crossing it says anything to the user, and why the two published over-quota log lines do not mean the same thing |

**One of these is a measurement, not a reading.** [`auto-backup-quota/`](./auto-backup-quota/README.md) resolves a question the documentation was already known to be silent on, so its primary evidence is an experiment run on the handset against the real cloud transport; §6 of that note reproduces it, and names two resets that must not be used.

Each report's own README carries its sources and confidence levels. The one-line gist of what each one settled lives in the map's Decisions-so-far.
