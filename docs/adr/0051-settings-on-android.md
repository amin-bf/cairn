# ADR-0051: Settings on Android — the development controls only in a development build, and no sync block

- **Status**: Accepted
- **Date**: 2026-10-30
- **Resolves**: [Native Android: What Settings Is on the Handset](https://github.com/amin-bf/cairn/issues/225)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the repository owner, 30.10.2026, accepting the agent's recommendation on each
  question and adding §1's build gate and §2's reinstall himself
- **Amends, on Android only**: [ADR-0042 §2 §3](0042-the-settings-screen.md) (what *This device*
  holds; the development controls *in every build*), [ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md)
  and [ADR-0015 §7](0015-the-sync-experience.md) (the empty collection's sentence, as ADR-0048 already
  reads it)
- **Related**: [ADR-0045 §5 §7](0045-the-native-android-client.md),
  [ADR-0047 §1](0047-the-file-surface-on-android.md) (the *Files* block),
  [ADR-0048 §1](0048-notes-and-the-editor-on-android.md) (*Settings* joins the destinations row),
  [ADR-0020 §4](0020-protection-at-rest.md) (the removal route, owed by sync settings)

## Context

ADR-0042 specifies Settings in terms that mostly carry to the Kotlin app unchanged: the *Studying*
group (the new-card rate and optimise, long work started from the foreground), the *This device* group,
Appearance (ADR-0036 §3), the *Files* block's *Open a file…* (ADR-0047 §1), and *Settings* joining
the destinations row (ADR-0048 §1). Two blocks had no Android reading.

The **Development** block holds reset, the fixture bench, the rendering specimen and the hand-off
specimen, drawn in every build (ADR-0042 §3, the owner's choice against the recommendation). Two of
the four are instruments of the egui stack. The **sync block** promises sync and opens an enrolment
sub-state whose only control is *Back*, because sync is built on no platform yet and is out of this
map's scope.

## Decision

### 1. The Kotlin app's development controls are reset and the fixture bench, and they exist only in a development build

> The *Development* block, at the end of Settings under its own heading, holds **reset** and the
> **fixture bench**, and nothing else. It is compiled into the app only when the build is made with a
> build-time environment variable set; built without it, the block and its code are not in the app at
> all, not merely hidden.

- **The fixture bench** stays because it is the handset's route to a fixture (`AGENTS.md` capture
  rule 21), and it already crosses the binding by name.
- **Reset** stays beside it as the way back to a first launch after a fixture.
- **The rendering specimen** is not carried. It shows how the egui renderer shapes each script, which
  says nothing about Compose; the debug build's design specimen is its Kotlin counterpart.
- **The hand-off specimen** is not carried. Half of it is the direct `MediaStore` write that ADR-0047
  replaced with the system save dialog. The other half, the share sheet, is checked against a real
  export in the file-surface slice.

This reverses ADR-0042 §3's *in every build* on Android only. A person who never sets the variable
never meets a control that replaces their collection. The desktop keeps ADR-0042 §3 as written.

The gate is a variable the build reads, not the build type. A development build can be a release
build, and an everyday build can be a debug one. The variable's name is the spec's to choose.

### 2. When development is finished, the handset gets a build made without the variable

> A session that installs a development build on the handset ends by installing a build made
> without the variable over it, so the app that stays on the phone has no *Development* block.

The reinstall is an **upgrade over the installed app**, never an uninstall: an uninstall deletes
`getFilesDir()`, which holds the owner's only copy of the collection. So both builds are signed with
the same key and share the package id `dev.cairn.app` (ADR-0045 §4). A signature mismatch makes
`install -r` refuse, and the tempting fix for that refusal is the uninstall that loses the collection.

### 3. The Kotlin app draws no sync block until sync is built

> *This device* holds the *Files* block and Appearance. No sync block, no *Set up sync* and no
> enrolment sub-state are drawn on Android. They return with the sync effort, which specifies them
> for a native client.

A control that leads nowhere is not drawn (the Review slice's rule). Nobody can be enrolled when they
upgrade from the egui build, because no build can enrol, so nothing visible is taken from anyone. The
removal route that sync settings must keep naming (ADR-0020 §4) binds once there is a sync setting
to name it, and nothing is published from Android before then.

### 4. The empty note list stops offering sync on Android

> On Android the empty collection reads *"Nothing here yet — create a deck, or import one"* until
> sync arrives. The desktop keeps *"…, or set up sync"*.

The sentence offers what the screen can reach. *Import one* stays because the file-surface slice
brings import.

## Amendments to accepted ADRs

All on Android only; the desktop reads each as written.

- **[ADR-0042 §3](0042-the-settings-screen.md)**: the development controls are reset and the fixture
  bench, in a development build only (§1).
- **[ADR-0042 §2](0042-the-settings-screen.md)**: *This device* holds the *Files* block and
  Appearance, with no sync block (§3).
- **[ADR-0021 §2](0021-note-ordering-saving-and-the-note-list.md)** and
  **[ADR-0015 §7](0015-the-sync-experience.md)**: the empty collection's sentence drops its sync
  clause (§4).

## Consequences

- The Settings spec round collects ADR-0042 as amended here, ADR-0011 (the rate), ADR-0014 and
  client-stack rule 10 (optimise), ADR-0036 §3 (appearance) and ADR-0047 §1 (*Open a file…*, whose
  picker comes with the file-surface slice).
- Handset judgements that install fixtures need a development build, and their tickets end with the
  §2 reinstall. Emulator tests are unaffected: they install fixtures through the binding, not through
  Settings.
- Reset does not cross the binding yet; the spec adds it beside `install_fixture`. Both stay in
  every build of `cairn-ffi`, and only the Kotlin block that reaches them is gated. Whether they
  should leave the shipped binding too is for a later ticket, if one is ever needed.
