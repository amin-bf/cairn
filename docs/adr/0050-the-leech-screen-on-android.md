# ADR-0050: The leech screen on Android — a place behind Review, a confirmed delete, and one count

- **Status**: Accepted
- **Date**: 2026-10-30
- **Resolves**: [Native Android: Spec Round — the Leech Screen](https://github.com/amin-bf/cairn/issues/224)
  (the decisions its spec needed that no earlier ADR made)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the agent writing the spec, under [ADR-0045 §7](0045-the-native-android-client.md)
  and the map's standing instruction to keep the owner's decisions to a minimum (29.10.2026). Each
  section follows from an accepted ADR or from Android's own practice; none was put to the owner.
- **Amends, on Android only**: [ADR-0010 §7](0010-leeches.md) (*Delete remains available and
  unchanged*)
- **Corrects, on both renderers**: what the durable entrance counts as a leech (§3)
- **Related**: [ADR-0010 §4 §6 §7 §8](0010-leeches.md), [ADR-0043](0043-the-leech-screen.md),
  [ADR-0021 §1 §5](0021-note-ordering-saving-and-the-note-list.md),
  [ADR-0046](0046-the-line-between-kotlin-and-rust.md), [ADR-0048 §1 §4 §6](0048-notes-and-the-editor-on-android.md)

## Context

ADR-0010 and ADR-0043 specify the leech screen in terms that carry to any renderer: the ranked list,
its three worded actions, the suspended section, the cost line, *Back to review* on the reach line, the
end-of-session pointer and the durable entrance. ADR-0048 settled where a phone's own input meets the
note list and the editor. Three things it left open come back on this screen: where the screen sits on
a back stack, whether its *Delete* is confirmed like the note list's, and one fault in the egui build
that the Kotlin app would otherwise copy.

## Decision

1. **The leech screen is a place on Review's back stack, not a destination.** ADR-0021 §1 keeps it
   off the destinations row: it hangs off Review, reached from the durable entrance and from the
   pointer's *Show me*. The row keeps *Review* marked while it shows. *Back to review* and the system
   Back are the same exit, as *Done* and Back are in the editor (ADR-0048 §6). *Edit* pushes the
   editor on top of it, and the return re-reads the list, so an edit that clears a leech or stops a
   card generating shows at once.

2. **The row's *Delete* asks first, in the row, with the note list's sentence.** *Delete ⟨preview⟩? It
   cannot be undeleted.* with *Delete* and *Keep it*, ⟨preview⟩ being **the note's** preview, the one
   the note list shows for it, bidi-isolated. ADR-0048 §4's reasons hold here unchanged: the delete
   discards content with no undelete, and a 36 dp control in a scrolling list takes stray taps under a
   thumb. The note is named rather than the card's prompt because the note is what goes, every one of
   its cards with it; one note, one question, whichever screen it is asked from. This amends ADR-0010
   §7's *unchanged* on Android only; the desktop keeps its immediate delete.

3. **The entrance counts the leeches the screen lists, and a suspended leech is named once.** The
   egui entrance counted every card over the floor, suspended ones included, while the screen lists a
   suspended card only in its suspended section — so a suspended leech was counted under both words
   of *Leeches (3) · suspended (1)* and the screen showed two. ADR-0010 §8 gives a suspended card one
   home, and the entrance's job is to say what that home holds. The count moves into `cairn-ops` as
   the active leeches (over the floor and not suspended), and both renderers read it.

4. **The pointer lives as long as the Review screen's state, and no longer.** ADR-0010 §6 stores
   nothing: the pointer is the leeches that crossed during the sitting just finished. On Android it is
   held by Review's ViewModel from the sitting's end until *Show me* or *Not now*; a rotation keeps it,
   and a process death drops it, as it drops the sitting (ADR-0006 §6). Nothing is written to the
   store or to instance state.

**Everything else carries as specified**: ranked worst first, a well per card with *Edit*, *Suspend*
and *Delete* in words at the ordinary weight and the touch target, the cost line, the suspended
section with *Edit* and *Unsuspend*, the empty sentence, the pointer's and the entrance's words and
their primary weight (ADR-0034 §2), and *Back to review* on the reach line.

## Consequences

- `cairn-ops` gains the leech screen's list and its two writes, *suspend* and *unsuspend*, which the
  egui screen makes straight on the store today; both renderers call them (ADR-0046 §1). *Delete*
  is the note list's operation.
- The desktop changes in one visible way: the entrance's leech count (§3).
- ADR-0010 §8's *suspension is visible on the card itself* has no build on either renderer; parity
  does not add it, and this ADR does not decide it.
