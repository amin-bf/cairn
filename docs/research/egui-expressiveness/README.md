# What the renderer can actually draw — egui 0.35.0, capability by capability

**Research ticket:** [#123](https://github.com/amin-bf/cairn/issues/123) (under wayfinder map
[#121, *Map: The Cairn Design Pass*](https://github.com/amin-bf/cairn/issues/121)) · **Date of
research:** 2026-08-08

**Question:** the design map has just opened seven things the system currently refuses — motion,
elevation, corner radius, the type scale, spacing, icons, and artwork in empty states. Whether the
renderer can *draw* them is a fact, not a decision, and every one of those decisions is worthless if
it turns out to be unexpressible. For each: **can it be drawn, at what call site, and at what cost**
— a `Visuals`/`Style` field, a widget-level override, or hand painting.

This is a **research** note. It gathers facts and sharpens trade-offs; it decides nothing. What the
design should do with any of this is a later ticket's job.

## Scope, sources, and why the version matters

Everything below is read from the **exact sources the lockfile pins**, unpacked on disk in the crate
registry: `egui 0.35.0`, `epaint 0.35.0` (the shape/tessellation/text layer egui paints through),
`emath 0.35.0` (its geometry and interpolation crate), `ecolor 0.35.0` (colour), `eframe 0.35.0` (the
windowing and rendering host). `Cargo.lock` pins all five at `0.35.0`, alongside `egui-winit 0.35.0`,
`egui_glow 0.35.0`, `egui-wgpu 0.35.0` and `winit 0.30.13`.

**The version discipline is not pedantry.** This library renames types across minor releases — the
corner-radius type was called `Rounding` in earlier versions and is `CornerRadius` here; the
context-level style setter that older documentation calls `Context::set_style` **does not exist** in
0.35.0 at all (§4.5). A fact carried over from another minor version is a wrong answer, so every
claim below cites `file:line` in the on-disk source. Where a claim is reasoning rather than reading,
it is marked **inference**.

**Three source caveats, all verified:**

- **The extras crate of §6 is not on disk, because it is not a dependency of this workspace.** Its
  feature list and dependency edges were read from the registry's own metadata for version `0.35.0`
  exactly, and its costs were measured by resolving scratch packages against the live registry — not
  taken from documentation for an unpinned version.

- **The packaged crates carry no changelog.** `egui-0.35.0/`, `epaint-0.35.0/` and `eframe-0.35.0/`
  as published contain only `Cargo.toml`, `README.md` and `src/` (plus `data/` and `benches/`).
  Nothing was quotable from a changelog and nothing was invented in its place.
- **The vendored dependency does not bear on any of this.** `vendor/egui-winit` is a verbatim copy of
  the published `egui-winit 0.35.0` carrying exactly one change: an `#[cfg(not(target_os =
  "android"))]` attribute on the block that interrupts input-method composition by toggling the
  window's IME-allowed flag off and on (`vendor/PATCH.md`; ADR-0026; `AGENTS.md` client-stack rule
  12). It touches soft-keyboard behaviour on Android and nothing about fonts, images, textures,
  shapes or rendering. **What it does bear on is the cost of moving off 0.35.0**: the patch is bound
  to the shape of that block, so a version bump is a re-judgement rather than a version-number edit —
  which makes "wait for a later release to make this expressible" an expensive answer to any question
  below.

Where the app is cited, it is this worktree's tree at `ad29b4e8`.

---

## The headline

**Six of the seven are expressible, and only one is genuinely blocked.**

- **Motion, elevation, corner radius, type, spacing** are all real, ambient, `Style`/`Visuals`-level
  capabilities — no hand painting required, no widget re-implementation.
- **Icons and empty-state artwork** are expressible by four different routes with very different
  costs, ranging from *no new dependency at all* to *fifty-nine new crates including a TLS stack*.
- **The one thing that cannot be done is synthetic bold** — the renderer has no emboldening anywhere,
  which is why the app already ships bold as its own family of real bold cuts.

But **the seven do not divide the way ADR-0030 §1 divides colour**, and that is the decision-relevant
result. Colour is ambient because every screen already asks for a *role* (`ui.visuals().text_color()`)
rather than a value. Of the seven:

- **Corner radius** is already ambient and already state-reactive, for free.
- **Type** is ambient *if* the scale is expressed as text styles, and the app already does that.
- **Spacing** is ambient in principle and **is named at ~60 call sites today**, so making it ambient
  is a migration, not a switch.
- **Motion** and **elevation** have **no ambient roles at all** for ordinary widgets. Every use is a
  screen naming a value. That is a cost, not a free choice.
- **Icons and artwork** are necessarily named at the call site — an icon is content, and no ambient
  mechanism exists to attach one to a widget.

§7 states that classification precisely.

---

## 1. Motion

### 1.1 What exists

Seven entry points, all on `Context`, all in the animation block starting at
`egui-0.35.0/src/context.rs:3078`:

| Signature | Line |
|---|---|
| `animate_bool(&self, id: Id, value: bool) -> f32` | `:3089` |
| `animate_bool_responsive(&self, id: Id, value: bool) -> f32` | `:3099` |
| `animate_bool_with_easing(&self, id: Id, value: bool, easing: fn(f32) -> f32) -> f32` | `:3105` |
| `animate_bool_with_time(&self, id: Id, target_value: bool, animation_time: f32) -> f32` | `:3112` |
| `animate_bool_with_time_and_easing(&self, id, target_value, animation_time, easing) -> f32` | `:3129` |
| `animate_value_with_time(&self, id: Id, target_value: f32, animation_time: f32) -> f32` | `:3162` |
| `clear_animations(&self)` | `:3180` |

`Ui` dereferences to `Context` (`egui-0.35.0/src/ui.rs:91`), so `ui.animate_bool(…)` is the same
function reached from a screen; there is no separate `Ui` animation API.

Every one of them returns a single `f32`. **That is the whole of the motion system**: it is a clock,
not a transition engine. There is no property animation, no keyframe list, no declarative
"transition: 200ms" attached to a widget. What the `f32` drives is whatever the caller interpolates
with it.

### 1.2 What the f32 can drive

Everything below is a real helper in the pinned sources, so nothing here needs hand-rolled maths:

- **Colour.** `Color32::lerp_to_gamma(other, t)` — `ecolor-0.35.0/src/color32.rs:356`. Note there is
  **no plain `Color32::lerp`**; the only interpolator blends in gamma space, which is the space the
  library's own styling works in. For perceptually-even blending, `Rgba` implements addition and
  scalar multiplication (`rgba.rs:222`, `:250`, `:264`) so the generic `emath::lerp` applies to it.
  `Color32::gamma_multiply(factor)` (`:294`) is the fade primitive.
- **Geometry.** `emath::lerp` (`emath-0.35.0/src/lib.rs:106`) works on `f32`, `f64` and `Vec2`;
  `Pos2::lerp` (`pos2.rs:201`) and `Rect::lerp_towards` (`rect.rs:462`) cover points and rectangles.
- **The opacity of a whole subtree**, which needs no lerping at all: `Ui::set_opacity` /
  `multiply_opacity` (`egui-0.35.0/src/ui.rs:560`, `:567`) and the same pair on `Painter`
  (`painter.rs:91`, `:100`). This is how the library's own window fade-out and floating-area fade-in
  work (`containers/window.rs:627`, `containers/area.rs:632-642`).
- **Whole-layer transforms** without re-laying-out: `Context::set_transform_layer` over a paint list
  (`egui-0.35.0/src/layers.rs:178`). *Inference:* this is the cheapest possible motion, because it
  post-multiplies shapes that have already been built.

What it **cannot** drive smoothly is a shadow, because the shadow's fields are integers — see §2.1.

### 1.3 Easing: yes, twenty-one curves, but a restricted signature

`emath-0.35.0/src/easing.rs` (declared `pub mod easing` at `emath-0.35.0/src/lib.rs:29`, reachable as
`egui::emath::easing`) carries twenty-one functions: `linear`, plus in/out/in-out triples for
quadratic, cubic, sine, circular, exponential, back (overshoot) and bounce. There is no elastic curve
and **no cubic-Bézier constructor** — a designer-authored easing curve of the kind other systems
express as four control-point numbers cannot be expressed here without writing the function.

Three constraints on using them:

1. **The easing parameter is a bare function pointer, `fn(f32) -> f32`**, not a closure type
   (`context.rs:3105`, `:3129`). A closure that captures anything will not compile. A parameterised
   curve has to be a named `fn`.
2. **`animate_bool` is linear.** `context.rs:3090-3091` passes `easing::linear` explicitly. So does
   `animate_bool_with_time` (`:3113-3118`). **`animate_value_with_time` has no easing parameter at
   all** and is linear only. The single entry point that picks a curve for you is
   `animate_bool_responsive`, which uses `cubic_out` (`context.rs:3100`).
3. **The curve is mirrored on the way back** — `context.rs:3150-3154` computes
   `if target { easing(t) } else { 1.0 - easing(1.0 - t) }`. An ease-out going in is an ease-in coming
   out, so a transition is visually symmetric whether you want that or not.

**One recorded defect, worth knowing before anyone picks a curve:** `sin_in` and `sin_out`
(`easing.rs:77`, `:85`) are computed over a *full* cycle (`2π`) where a quarter-cycle (`π/2`) is
wanted, so `sin_out(1.0) = 0` — the curve does not end where it started from and is not monotonic.
`sin_in_out` (`:93`) is correct. Use the quadratic or cubic families.

### 1.4 What a transition costs in a repaint-on-demand loop

This is the load-bearing part, because this app repaints on demand.

**The animation call requests its own repaints.** `context.rs:3145-3148`, inside the function every
`animate_bool*` routes through:

```rust
let animation_in_progress = 0.0 < animated_value && animated_value < 1.0;
if animation_in_progress {
    self.request_repaint();
}
```

The inequalities are strict and the underlying manager clamps to exactly `0.0`/`1.0`
(`animation_manager.rs:58`), so the repaint stream stops cleanly when the animation lands.
`animate_value_with_time` does the same on `animated_value != target_value` (`:3171-3174`).

**A zero-delay repaint request is a request for two frames, not one.** `context.rs:137-139` sets an
`outstanding` counter with the comment *"Each request results in two repaints, just to give some
things time to settle."* A *delayed* request (`request_repaint_after`) does **not** set that counter
— it only lowers a delay, and the smallest requested delay wins (`:140-166`). So
`request_repaint_after_secs` schedules one wake-up; `request_repaint` starts a stream.

**The frames are not cheap, because nothing is cached between them.** `Context::tessellate`
(`context.rs:2757`) carries the explicit note at `:2764-2766`:

> *"A tempting optimization is to reuse the tessellation from last frame if the shapes are the same,
> but just comparing the shapes takes about 50% of the time it takes to tessellate them, so it is not
> a worth optimization."*

A fresh tessellator is built per call (`:2784-2790`). So every animation frame re-runs the whole
`update` closure, re-lays out every text run, re-tessellates every shape on screen, and re-uploads.

**Inference, from those three facts together:** a single in-flight animation pins the entire viewport
at the display's frame rate for its duration. At the default `animation_time` of `0.2 s` and 60 Hz
that is roughly **twelve full-application layout-and-tessellation passes per transition**, and it is
per-context, not per-widget — one animating control repaints every screen element. On the handset
that is the cost to weigh; on the desktop it is a rounding error. `Context::repaint_causes()`
(`context.rs:1878`) is the instrument for finding an animation that never settles, because every
`animate_*` is `#[track_caller]` and records the calling line.

**The app already keeps one screen ticking.** `crates/app/src/screens/review.rs:113` calls
`request_repaint_after(Duration::from_secs(1))` for the whole time a sitting is running, so that the
ten-minute checkpoint can surface without an input event. That is the cheap, delayed form — one
scheduled wake-up per second, not a stream — and it is already in the budget. The other repaint call
sites are one-shots: `lib.rs:396` (after the deferred font install), `keyboard.rs:70`, and
`screens/settings.rs:661`, `:707` (polling a worker thread, per client-stack rule 4).

### 1.5 `Style::animation_time` is one global knob, and a screen cannot override it

`egui-0.35.0/src/style.rs:317` — `pub animation_time: f32`, defaulting to **`0.2`** (`:1434`). It is
on `Style`, not `Visuals` and not `Spacing`.

The complete set of readers is four: `animate_bool` (`context.rs:3090`), `animate_bool_with_easing`
(`:3106`), the floating-area fade-in (`containers/area.rs:636`), and the built-in style editor
(`style.rs:1886`).

**The first two read `ctx.global_style()`, not the current `Ui`'s style.** So
`ui.style_mut().animation_time = …` inside a scope has *no effect* on `animate_bool` — the value is
read from the context-level style regardless. There is no per-widget or per-screen duration. The only
way to vary a duration is to call `animate_bool_with_time(…)` and pass the number, which is a screen
naming a value.

The one adjacent knob is `Style::scroll_animation` (`style.rs:827-833`) — `{ points_per_second: f32`
(default `1000.0`)`, duration: Rangef` (default `0.1..=0.3`)`}` — read only by the scroll area.

### 1.6 Animation state: keyed by `Id`, never collected, and it snaps on first sight

`egui-0.35.0/src/animation_manager.rs`, 110 lines, read in full.

State lives in two maps keyed by widget `Id` (`:6-10`), each an integer-keyed hash map with **no
capacity bound and no eviction**. Per boolean it stores `{ last_value: f32, last_tick: f64 }`
(`:12-16`).

Three consequences that decide how motion has to be written:

1. **The first frame for an `Id` snaps to the target.** The `None` branch (`:38-48`) inserts the end
   value and returns it. So an animation only ever *runs* from the second frame an `Id` is seen.
   **Inference:** any `Id` derived from something unstable — a list index that shifts, a string that
   the user is editing, a value recomputed per frame — creates a fresh entry every frame, snaps every
   frame, and never tweens. Nothing fails; the motion is simply absent. This is the predictable way
   to "lose" an animation here.
2. **Nothing is ever garbage-collected.** The file contains `insert` and `get_mut` and no `remove`,
   no `retain`, no age field, and the manager is untouched by the per-pass begin/end hooks. (The
   memory store *does* prune, `memory/mod.rs:716`, `:772-774` — the animation store does not.) The
   only way to free it is `Context::clear_animations()`, which replaces the whole thing. *Inference:*
   animating per-item with a unique `Id` per note or per card leaks a small fixed record per `Id` for
   the life of the process. Small; unbounded.
3. **A widget not shown for a frame pauses rather than jumps.** The elapsed time is clamped to one
   frame's worth: `let elapsed = ((current_time - *last_tick) as f32).at_most(input.stable_dt);`
   (`:54-56`). So a control hidden mid-transition and shown ten seconds later resumes from where it
   stopped, advancing one frame at a time. It will read as *paused*, not as *skipped*. And while
   hidden it also requested no repaints, so nothing was driving it.

A fourth, useful: `animation_time == 0.0` divides by zero, the result is non-finite, and the manager
snaps to the target (`:57-61`). The library uses that deliberately to force a snap while a panel is
being dragged (`containers/panel.rs:533`).

### 1.7 Nothing in `widgets/` animates today

A search for `animate_bool*` outside `context.rs` returns **zero hits under
`egui-0.35.0/src/widgets/`**. Every use is in a container: panel expand/collapse
(`containers/panel.rs:26`), collapsing-header openness (`collapsing_header.rs:78`), scrollbar
show/hide and hover thickening (`scroll_area.rs:752`, `:1345`, `:1473`), window fade-out
(`window.rs:627`), floating-area fade-in (`area.rs:632`).

Two widgets animate by reading the clock directly and requesting a repaint unconditionally every
frame, bypassing the manager: the spinner (`widgets/spinner.rs:38-40`) and the progress bar when
`.animate(true)` is set (`widgets/progress_bar.rs:82`, `:131`).

**So hover, press and focus feedback on buttons, checkboxes and fields is instantaneous by default.**
The state-specific visuals are swapped, not blended. Any fade the design wants on a widget is
something the app writes.

---

## 2. Elevation and shadow

### 2.1 What a shadow is

`epaint-0.35.0/src/shadow.rs`, 85 lines, read in full:

```rust
pub struct Shadow {
    pub offset: [i8; 2],   // :15
    pub blur: u8,          // :20  — the width of the fuzzy penumbra; 0 is a sharp edge
    pub spread: u8,        // :23  — expand in all directions before blurring
    pub color: Color32,    // :26
}
```

**Every geometric field is an integer.** The offset is a signed byte per axis (−128..=127 points),
blur and spread are unsigned bytes (0..=255). The struct is eight bytes and a test pins that
(`:29-36`). It derives `Eq`, which is only possible because nothing in it is floating point.

There are exactly three items besides the fields: the constant `NONE` (`:40`, all-zero and
transparent), `as_shape(rect, corner_radius) -> RectShape` (`:48`), and `margin() -> MarginF32`
(`:68`), which reports how far past its caster the shadow reaches.

*Inference:* integer typing means a shadow **cannot be animated smoothly by its geometry**. A blur
ramp from 0 to 10 is ten visible steps. A smooth elevation change has to be driven through the
`color`'s alpha, which is continuous.

### 2.2 How it is actually rasterised — a widened anti-aliasing feather, not a blur

`Shadow::as_shape` (`shadow.rs:57-64`) translates the caster's rectangle by the offset, expands it by
the spread, adds the spread to the corner radius, and produces **one filled rounded rectangle** whose
`blur_width` is the blur.

That field's own documentation, `epaint-0.35.0/src/shapes/rect_shape.rs:44-50`, says what it is:

> *"If larger than zero, the edges of the rectangle (for both fill and stroke) will be blurred. …
> The blur is currently implemented using a simple linear blur in sRGBA gamma space."*

And the implementation, `epaint-0.35.0/src/tessellator.rs:1863-1877`, is explicit:

```rust
if self.feathering < blur_width {
    // We accomplish the blur by using a larger-than-normal feathering.
    // Feathering is usually used to make the edges of a shape softer for anti-aliasing.
    ...
    corner_radius += 0.5 * blur_width;
    self.feathering = self.feathering.max(blur_width);
}
```

**So the shadow is the same edge-softening gradient the renderer applies to every shape, simply
widened.** It is a *linear* alpha ramp across the edge, computed in gamma space — not a Gaussian, not
a separable blur, not a render-to-texture pass. *Inference:* at large blur values it therefore reads
harder-edged and more banded at the penumbra than a true Gaussian drop shadow with the same numbers.
Softer elevation comes from lowering the alpha, not from raising the blur.

Two clamps come with that mechanism, both silent:

- **Blur is clamped to the caster's smaller side** (`:1870-1872`, `min_elem() - 0.1 - 2*stroke.width`).
  A 15-point blur under a 20-point-tall control is quietly reduced. Shadows under small elements are
  weaker than asked for.
- **The corner radius grows by half the blur** (`:1874`), on top of the spread already added in
  `as_shape`. A spread-and-blurred shadow is visibly rounder than the thing casting it.

**The fill cost.** With feathering on, the closed-path filler reserves `3n` triangles and `2n`
vertices — an inner ring and a fully transparent outer ring (`tessellator.rs:777-806`) — against
`n − 2` triangles unfeathered (`:808-816`). `n` comes from the rounded-rectangle path builder, which
picks a corner resolution by radius: 3 points per corner up to radius 2, 5 up to 5, 9 below 18, 17
below 50, 33 above (`:604-632`). *Inference, doing that arithmetic for the stock window shadow* (blur
15, radius 6 → effective 13.5, so nine points per corner): roughly **36 path points, ~108 triangles,
~72 vertices per shadow.** Trivial for the GPU. The real cost is that it is CPU-tessellated from
scratch **every frame** (§1.4), in the same draw batch as everything else — no texture, no extra
pass, no shader.

### 2.3 Where a shadow is honoured — **any frame, not only windows**

This is the question the ticket asked, and the answer is the permissive one.

`egui::containers::Frame` in 0.35.0 has six fields (`containers/frame.rs:96-141`): `inner_margin`,
`fill`, `stroke`, `corner_radius`, `outer_margin`, and **`shadow: Shadow`** (`:140`, *"Optional
drop-shadow behind the frame."*). It has a builder, `Frame::shadow(shadow)` (`:303`). And the code
that paints it (`:423-449`) is completely generic:

```rust
if shadow == Default::default() { frame_shape }
else {
    let shadow = shadow.as_shape(widget_rect, corner_radius);
    Shape::Vec(vec![Shape::from(shadow), frame_shape])
}
```

Nothing there asks whether the frame belongs to a window. **`Frame::new().shadow(s).show(ui, |ui|
…)` puts a shadow under arbitrary content**, including a single widget.

What is *ambient* is narrower. `Visuals` carries exactly two shadow fields —
`window_shadow` (`style.rs:1058`) and `popup_shadow` (`:1070`) — and only three frame constructors
read them: `Frame::window` (`frame.rs:200`), `Frame::menu` (`:209`) and `Frame::popup` (`:218`).
**There is no shadow on `WidgetVisuals`**, so no ordinary widget picks one up from the theme. The
one place a widget-level `Frame` gains a `shadow` field is the new per-widget style layer of §3.4,
and it is set to `Shadow::NONE` there and never populated from `Visuals`
(`egui-0.35.0/src/widget_style.rs:202`).

Stock defaults, for reference — geometry identical between themes, only the alpha differs:

| | offset | blur | spread | colour |
|---|---|---|---|---|
| `dark().window_shadow` (`style.rs:1512`) | `[10, 20]` | 15 | 0 | black at alpha 96/255 |
| `dark().popup_shadow` (`:1526`) | `[6, 10]` | 8 | 0 | black at alpha 96/255 |
| `light().window_shadow` (`:1574`) | `[10, 20]` | 15 | 0 | black at alpha 25/255 |
| `light().popup_shadow` (`:1585`) | `[6, 10]` | 8 | 0 | black at alpha 25/255 |

**The app sets both to `Shadow::NONE` today** — `crates/app/src/theme.rs:142-143`, under the comment
*"No shadow anywhere: a Cairn surface is a fill and a 2px corner, and nothing floats."* That is a
palette-level decision the design pass may revisit, and it is expressed exactly where an ambient
value belongs.

### 2.4 Putting a shadow under an arbitrary widget — two routes and one hazard

**The supported route** is a wrapping frame: `Frame::new().shadow(s).corner_radius(r).show(ui, |ui| {
ui.add(widget) })`.

**The hand route**, for when you do not know the rectangle until after the widget has drawn, is the
same trick the frame uses internally. Reserve a slot in the paint list, draw, then fill the slot in:

```rust
let idx = ui.painter().add(egui::Shape::Noop);          // painter.rs:213 -> ShapeIdx
let response = ui.add(my_widget);
ui.painter().set(idx, shadow.as_shape(response.rect, r)); // painter.rs:242
```

`ShapeIdx` is at `egui-0.35.0/src/layers.rs:109`; the paint list's own documentation spells the idiom
out at `:143-148`. `Painter::set` takes `&self`, so `ui.painter().set(…)` is fine. Two caveats:
`set` also replaces the shape's clip rectangle with the painter's *current* one (`layers.rs:149-157`),
and an out-of-range index warns and does nothing (`:150-153`). If the rectangle is known in advance,
no reservation is needed at all — the paint list is strictly back-to-front insertion order, so
painting the shadow before the widget is enough. A third route exists too: `Frame::begin` returns a
`Prepared` whose `frame` field is public and mutable (`frame.rs:357-369`), so the shadow can be
decided *after* seeing the content.

**The hazard, and it is silent.** `Frame::total_margin()` (`frame.rs:327-331`) sums inner margin,
stroke width and outer margin — and **not** the shadow. `Shadow::margin()`, which exists precisely to
report the penumbra's reach, **has no callers anywhere in the two crates.** So a shadowed frame in an
ordinary layout reserves no room for its own shadow: the penumbra paints outside the allocated
rectangle, overlaps whatever sits below and to the right, and gets **clipped** at the boundary of the
enclosing scroll area or panel. Windows and popups escape this only because they live on their own
layer. The fix is to set `outer_margin` to roughly `shadow.margin()` by hand — which is a screen
naming a value derived from another value it also named.

A second silent one: the skip test is `shadow == Default::default()` (`frame.rs:443`), i.e. *all*
fields zero. A shadow faded to a transparent colour but with a non-zero blur is not equal to the
default, so it still emits its ~108-triangle shape every frame. Animate to `Shadow::NONE`, not merely
to alpha zero.

---

## 3. Corner radius

### 3.1 The type, and its integer floor

`epaint-0.35.0/src/corner_radius.rs:13-25`:

```rust
pub struct CornerRadius { pub nw: u8, pub ne: u8, pub sw: u8, pub se: u8 }
```

**The name is `CornerRadius` in 0.35.0** — `Rounding` does not exist anywhere in these crates. The
scalar is `u8`, and the type's own documentation says why (`:7-8`): *"The rounding uses `u8` to save
space, so the amount of rounding is limited to integers in the range `[0, 255]`."*

So **a radius is a whole number of points. There is no 2.5px corner.** A companion `CornerRadiusF32`
exists (`corner_radius_f32.rs:8-20`) for intermediate arithmetic, and converting back rounds
(`:34-44`). `From<f32> for CornerRadius` also rounds (`corner_radius.rs:41-46`), which is why both
`.corner_radius(3)` and `.corner_radius(3.0)` compile — the second is not more precise, it is the
same value with a rounding step in front of it.

### 3.2 Per-corner granularity: yes, by struct literal

The helpers are `ZERO` (`:50`), `same(u8)` (`:59`), `is_same` (`:70`), `at_least`/`at_most`
(`:76`, `:87`) and `average` (`:97`). There is **no** four-argument constructor and no per-corner
builder — but all four fields are public, so an asymmetric radius is written directly:

```rust
CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }
```

Arithmetic on it saturates rather than wrapping (`:102-200`).

### 3.3 Per-widget-state: it is a `Visuals` field, and the 2→3 hover change is stock

`WidgetVisuals` (`egui-0.35.0/src/style.rs:1289-1313`) carries `corner_radius: CornerRadius` at
`:1302`, alongside `bg_fill`, `weak_bg_fill`, `bg_stroke`, `fg_stroke` and `expansion`. `Visuals::widgets`
(`:1026`) is a `Widgets` struct with the five states `noninteractive`, `inactive`, `hovered`,
`active`, `open` (`:1244-1264`).

The defaults, from `Widgets::dark()` (`:1673-1716`) and `Widgets::light()` (`:1718-1761`):

| state | dark | light |
|---|---|---|
| `noninteractive` | `same(2)` (`:1680`) | `same(2)` (`:1725`) |
| `inactive` | `same(2)` (`:1688`) | `same(2)` (`:1733`) |
| **`hovered`** | **`same(3)`** (`:1696`) | **`same(3)`** (`:1741`) |
| `active` | `same(2)` (`:1704`) | `same(2)` (`:1749`) |
| `open` | `same(2)` (`:1712`) | `same(2)` (`:1757`) |

**This answers the ticket's question directly and with a result worth stating plainly: the 2px→3px
hover change is a `Visuals` field, read ambiently by every widget — and it is the renderer's stock
default, not something anyone chose.** The app's `crates/app/src/theme.rs:107-132` restates those
same numbers while renaming the fills and strokes, so the corner behaviour Cairn draws today is
inherited, not decided. The map's test — *"does it say something the app cannot know, or is it only
how the app is drawn?"* — puts this squarely in the second category, and it is the cheapest of the
seven to change: five numbers in one file.

Two further ambient radii sit on `Visuals` directly: `window_corner_radius` (`:1057`, default
`same(6)` at `:1511`) and `menu_corner_radius` (`:1065`, default `same(6)` at `:1522`).

### 3.4 How widgets actually reach it — and a new style layer in 0.35

0.35.0 introduces a per-widget style-resolution layer that older versions did not have:
`egui-0.35.0/src/widget_style.rs`. It defines `WidgetState { Noninteractive, Inactive, Hovered,
Active }` (`:84-90`), `Response::widget_state()` (`:105-115`), and resolvers on `Style` —
`widget_style` (`:120`), `button_style` (`:146`), `checkbox_style` (`:174`), `label_style` (`:194`),
`separator_style` (`:212`) — each returning a small struct whose core is an `egui::Frame`. It also
carries a class system (`Classes`, `HasClasses`, `ROOT_CLASS`, `SELECTED_CLASS`, `:221-318`) whose
own documentation says it exists *"to be used by styling engine to compute a different style based on
the set of classes present"*. **Only `SELECTED_CLASS` is actually consulted today** (`:150`), and
there is no hook to substitute your own resolver — the resolvers are inherent methods on `Style`, not
a trait. *Inference:* this is a migration in progress toward a themeable widget layer; today it is
plumbing, not an extension point.

So a button obtains its radius like this (`widgets/button.rs:325-351`): it reads its own response
from the previous pass (`ui.ctx().read_response(id)`), derives a `WidgetState`, calls
`ui.style().button_style(&classes, state)` which copies `visuals.corner_radius` into a `Frame`
(`widget_style.rs:161`), and then applies its own optional override last. A checkbox goes the same
way (`checkbox.rs:83`). A text field still uses the older path, `ui.style().interact(&response)`
(`widgets/text_edit/builder.rs:695`).

The classic selector is still there and still readable: `Style::interact(&Response)` (`style.rs:354`)
delegating to `Widgets::style` (`:1267-1278`), which picks `noninteractive` when the widget is not
interactive, `active` when the pointer is down on it *or* it has focus *or* it was clicked, `hovered`
when hovered or highlighted, and `inactive` otherwise. Note that **`open` is never selected by that
function** — a caller must reach for it deliberately, and the new `Widgets::state()` API
(`widget_style.rs:94-101`) cannot reach it at all.

**Per-call-site radius overrides exist on exactly four things** in the whole library — `Frame`
(`frame.rs:277`), `Button` (`button.rs:200`), `Image` (`widgets/image.rs:251`), `ProgressBar`
(`progress_bar.rs:93`). A checkbox, a text field, a combo box and a slider have none. A text field
does have `TextEdit::frame(Frame)` (`builder.rs:303`), and supplying one **skips the entire ambient
branch** (`:692-727`) — so it is a total replacement of fill, stroke, margins and radius, not a
radius override.

---

## 4. Type

### 4.1 The slots are not a ceiling

`TextStyle` (`egui-0.35.0/src/style.rs:71-94`) has five built-in variants — `Small`, `Body`,
`Monospace`, `Button`, `Heading` — **plus `Name(Arc<str>)`**. `Style::text_styles` is a
`BTreeMap<TextStyle, FontId>` (`:288`), so arbitrary named slots are first-class: register
`TextStyle::Name("caption".into())` in the map and every call site can ask for it by name.

The one sharp edge: **resolution panics if the name was never registered.** `TextStyle::resolve`
(`:111-119`) does `text_styles.get(self).cloned().unwrap_or_else(|| panic!(…))`. The same is true of
font families — an unbound `FontFamily::Name` panics with *"is not bound to any fonts"*
(`epaint-0.35.0/src/text/fonts.rs:1029-1031`). A named style or family is therefore a contract
between one registration site and every reader, exactly like the palette; a typo is a crash, not a
fallback.

Stock sizes, from `default_text_styles()` (`style.rs:1408-1419`): `Small` 9.0, `Body` 13.0, `Button`
13.0, `Heading` 18.0, `Monospace` 13.0, the first four in the proportional family.

> **A discrepancy worth recording.** [ADR-0030 §3](../../adr/0030-the-first-finish-pass-decisions.md)
> states *"the body style is **12.5px** and the small style **9px**"*. Against 0.35.0 the small style
> is 9.0 as stated, but the body style is **13.0** (`style.rs:1413`), and the app overrides neither.
> Nothing downstream breaks — §3's argument is about 9px small text, which is correct, and the
> contrast measurements do not depend on the size — but the 12.5 figure does not match the version
> the binary is built against. *Confidence: high on the 0.35.0 value (read from source); the origin
> of 12.5 was not traced.*

### 4.2 An arbitrary size needs no registration

`FontId` (`epaint-0.35.0/src/text/fonts.rs:27-34`) is `{ size: f32, family: FontFamily }` — and the
source carries the telling comment on the next line: `// TODO(emilk): weight (bold), italics, …`.
**There is no weight and no slant on a font identity here.**

Any call site may name a size directly. `RichText::size(f32)` (`egui-0.35.0/src/widget_text.rs:148`),
`.family(FontFamily)` (`:185`) and `.font(FontId)` (`:193`, which is exactly the previous two
together) all work without registering anything, because the resolved identity is mutated in place
(`:425-438`):

```rust
let mut font_id = /* override_font_id, else text_style, else override_text_style, else fallback */;
if let Some(size) = size { font_id.size = size; }
if let Some(family) = family { font_id.family = family; }
```

The precedence is worth stating because it inverts what one might expect: `Style::override_font_id`
beats *everything*, including a call site's `text_style`; then the call site's `text_style`; then
`Style::override_text_style`; then the widget's own fallback. Size and family then overlay whatever
won.

The app takes the disciplined path and does not use per-call-site sizes for the type scale: every
helper in `crates/app/src/lib.rs` resolves a *slot* — `TextStyle::Button.resolve(ui.style())` at
`:590` and `:656`, `Heading` at `:598`, `Body` at `:606` and `:739`, `Small` at `:638` and `:666` —
and hands the resulting `FontId` to the bidi layout builder. The two places that name a raw size are
the development-only rendering specimen, `crates/app/src/screens/settings.rs:282` and `:287`
(`FontId::new(11.0, …)` and `FontId::new(20.0, …)`), which draws each shipped family against a
caption and is not a product surface.

### 4.3 There is no synthetic bold, and there is no place to hang one

Definitive, and it matters because the app's font module already records the consequence.

- **`TextShape` has no stroke.** Its complete field list (`epaint-0.35.0/src/shapes/text_shape.rs:12-42`)
  is position, galley, `underline: Stroke`, fallback colour, override colour, opacity factor, angle.
  The only stroke is the underline — an explicit horizontal line. There is no outline, no weight, no
  widening.
- **`TextFormat` has no bold flag.** Its fields (`text_layout_types.rs:471-516`) are font identity,
  letter spacing, line height, colour, background, background expansion, variation coordinates,
  `italics`, underline, strikethrough, vertical alignment.
- **`FontTweak` has no weight field.** Its nine fields (`fonts.rs:214-286`) are `scale`,
  `y_offset_factor`, `y_offset`, `hinting`, `hinting_target`, `subpixel_binning`, `coords`,
  `thin_space_width`, `tab_size`. `scale` makes a face bigger, not heavier.
- **Glyphs are pre-rasterised atlas quads**, one textured quad per glyph
  (`text/text_layout.rs:1155-1206`). There is no outline at that layer to dilate.

The one synthetic style that *does* exist is italics, and its mechanism shows how little machinery is
there: `text_layout.rs:1174-1200` shifts the two top vertices of each glyph quad by
`rect.height() * 0.25` horizontally. A shear, hand-applied, on the quad. **There is no bold
counterpart.**

This is exactly what `crates/app/src/fonts.rs:39-44` already records — *"There is no synthetic bold to
fall back on. epaint has no emboldening, and egui's own `RichText::strong` answers emphasis by
brightening the colour"* — and the source confirms both halves: `RichText::strong()`
(`widget_text.rs:252`) sets a boolean that is consumed only by the colour resolver
(`:481-491`, `Visuals::strong_text_color()`), and the flag is explicitly discarded from the format
(`:414-415`). Against a near-white body colour that is invisible, which is why ADR-0012 §8 rules
*bold is a face, never a colour*, and why `fonts::bold_family()` builds a third family from real bold
cuts (`fonts.rs:49-51`, `:132-141`).

*Inference on the one route not taken:* drawing a galley twice at a one-pixel offset would double the
vertex count, smear glyphs against the sub-pixel binning that is on by default
(`epaint-0.35.0/src/text/mod.rs:53`, default `true`), and — decisively — not widen the layout
advances, so the text would collide with its neighbours. It is not a fallback.

**What 0.35.0 does newly offer is variable-font axes**, which are a real alternative to shipping a
separate cut. `VariationCoords` (`text_layout_types.rs:411`) carries `(tag, value)` pairs;
`RichText::variation("wght", 700.0)` (`widget_text.rs:202`) sets one per call site; `FontTweak::coords`
sets a per-face baseline, and the two merge with the call site winning
(`epaint-0.35.0/src/text/font.rs:575-580`). The coordinates enter the glyph cache key, so different
weights get distinct atlas entries rather than fighting. **But it needs a variable font**: the four
bundled faces are static, and so are all four the app ships. Using this would mean replacing a shipped
face with a variable one — a real option for a future type decision, not a way to make the current
faces heavier.

### 4.4 What the font stack permits

`FontDefinitions` (`fonts.rs:437-450`) is two maps: `font_data: BTreeMap<String, Arc<FontData>>` and
`families: BTreeMap<FontFamily, Vec<String>>`, where each family's vector is an **ordered fallback
chain** — the first face that has a glyph draws it (`:445-448`). `FontFamily` is `Proportional`,
`Monospace`, or `Name(Arc<str>)` (`:80-101`), so arbitrarily many named families may be registered.
`FontData::from_static` (`:131`) takes bytes with no allocation, which is what the app uses.

The app's `crates/app/src/fonts.rs:110-164` does exactly this: four faces embedded with
`include_bytes!` and inserted under short names, the two regular cuts appended as fallbacks to
`Proportional` and `Monospace`, and the two bold cuts installed as the *sole* contents of a third
family named `"bold"`. The embedded weight is **1,964,992 bytes** across four files
(`crates/app/assets/`: `DejaVuSans.ttf` 759,720; `DejaVuSans-Bold.ttf` 708,920;
`NotoSansArabic-Bold.ttf` 261,460; `NotoSansArabic-Regular.ttf` 234,892).

### 4.5 The context-level setters, and the cost of changing fonts at runtime

**`Context::set_style` does not exist in 0.35.0.** The style API is `global_style()` (`context.rs:2107`),
`global_style_mut` (`:2121`), `set_global_style` (`:2132`), `all_styles_mut` (`:2145`), `style_of(Theme)`
(`:2153`), `style_mut_of` (`:2169`), `set_style_of` (`:2182`), `set_visuals_of` (`:2199`) and
`set_visuals` (`:2212`). The app's `theme::install` uses `set_visuals_of(Theme::Dark, …)` and
`set_theme(ThemePreference::Dark)` (`crates/app/src/theme.rs:87-90`) — the targeted setter, for the
reason its own module header gives.

**Changing the font set at runtime is a full teardown, and this is why the app installs once.**
`Context::set_fonts` (`:2038`) first compares the new definitions against the old, with the source's
own warning at `:2042-2043`: *"NOTE: this comparison is expensive since it checks TTF data for
equality"* — the definitions derive equality and the font payload is a byte slice, so this
byte-compares every embedded file. If they differ, the font system is dropped (`:537`) and rebuilt:
`Fonts::new` (documented *"This call is expensive, so only create one `Fonts` and then reuse it"*,
`epaint-0.35.0/src/text/fonts.rs:719-721`) re-parses every face, allocates a **new texture atlas**,
and starts a **fresh, empty layout cache**. The whole atlas texture is then re-uploaded and every
piece of text on screen must be laid out and tessellated again.

The same teardown fires without touching fonts if `Visuals::text_options` changes, or when the atlas
passes 80% full (`fonts.rs:734-749`). *Inference, worth flagging for a light palette:* the dark and
light stock visuals set **different** colour transfer functions (`style.rs:1494`, `:1561`), so a theme
switch forces a full atlas rebuild. That is a cost a future light-mode pass inherits rather than
introduces.

---

## 5. Spacing

### 5.1 The twenty fields

`Spacing` (`egui-0.35.0/src/style.rs:384-462`), with defaults from `:1446-1471`:

| field | type | default |
|---|---|---|
| `item_spacing` | `Vec2` | `(8.0, 3.0)` |
| `window_margin` | `Margin` | `same(6)` |
| `button_padding` | `Vec2` | `(4.0, 1.0)` |
| `menu_margin` | `Margin` | `same(6)` |
| `indent` | `f32` | `18.0` |
| `interact_size` | `Vec2` | `(40.0, 18.0)` |
| `slider_width` | `f32` | `100.0` |
| `slider_rail_height` | `f32` | `8.0` |
| `combo_width` | `f32` | `100.0` |
| `text_edit_width` | `f32` | `280.0` |
| `icon_width` | `f32` | `14.0` |
| `icon_width_inner` | `f32` | `8.0` |
| `icon_spacing` | `f32` | `4.0` |
| `default_area_size` | `Vec2` | `(600.0, 400.0)` |
| `tooltip_width` | `f32` | `500.0` |
| `menu_width` | `f32` | `400.0` |
| `menu_spacing` | `f32` | `2.0` |
| `indent_ends_with_horizontal_line` | `bool` | `false` |
| `combo_height` | `f32` | `200.0` |
| `scroll` | `ScrollStyle` | sixteen further fields, floating preset |

`interact_size.y` is the one the map's *"a 36px button stays 36px on desktop"* rule interacts with: a
button's minimum height is raised to it unless the button is marked small (`widgets/button.rs:307-309`),
so a touch-sized minimum is expressible as one ambient number rather than per-button sizing.

**`Margin` is four `i8`s** (`epaint-0.35.0/src/margin.rs:15-20`), with the same rationale as the corner
radius: *"All values are stored as `i8` to keep the size of `Margin` small."* So margins are whole
points, −128..=127, and converting from a `Vec2` both rounds and **collapses to symmetric**
(`:115`) — `Margin::from(Vec2)` builds `symmetric(x.round(), y.round())` and loses per-side asymmetry.
The `Vec2`-typed spacing fields keep full float precision; the `Margin`-typed ones do not.

### 5.2 Local override, and what it costs

The accessors are `ui.spacing()` / `ui.spacing_mut()` (`ui.rs:398`, `:411`), `ui.style()` /
`ui.style_mut()` (`:364`, `:379`), `ui.visuals()` / `ui.visuals_mut()` (`:418`, `:433`),
`ui.set_style()` (`:386`), `ui.reset_style()` (`:391`), `ui.scope()` (`:2185`), `ui.scope_builder()`
(`:2193`), `ui.push_id()` (`:2163`).

Every mutable accessor funnels through one line (`ui.rs:379-381`):

```rust
pub fn style_mut(&mut self) -> &mut Style {
    Arc::make_mut(&mut self.style) // clone-on-write
}
```

A child `Ui` inherits by reference-count bump — `let style = style.unwrap_or_else(|| Arc::clone(&self.style));`
(`ui.rs:236`) — so the count is at least two and **the first mutable access in a fresh scope deep-copies
the whole `Style`**: twenty spacing fields including a sixteen-field scroll style, the interaction
settings, the whole visuals tree with its five widget states, and — the only allocating part — the
`BTreeMap` of text styles. Subsequent mutations in the same `Ui` are free, because the count is now
one.

*Inference:* one style clone plus one map allocation per overriding `Ui` per frame is negligible for a
handful of scopes and measurable only inside a tight loop over thousands of rows — where the fix is to
hoist the override to one enclosing scope rather than to avoid scoping.

**A scope cannot leak.** `Ui::scope` (`:2202-2214`) builds a child, runs the closure against it, and
afterwards reads back only geometry. The parent's style field is never written, so nothing needs
restoring — the isolation is structural, not a save/restore pair. `UiBuilder::style`
(`ui_builder.rs:155`) replaces inheritance outright, and `StyleModifier` (`style.rs:193`) packages a
mutation as a shareable value.

### 5.3 What the app does today, and why this one is a migration

**The app names spacing at the call site, everywhere, and uses no ambient spacing at all.** There is
not one `ui.spacing_mut()` in `crates/app/src/`. Instead there are roughly sixty literal
`ui.add_space(N)` calls with N in {4, 8, 10, 12, 16} — for instance `screens/review.rs:57`, `:59`,
`:67`, `:74`, `:99`, `:109`, `:136`, `:155`, `:164`, `:171`, `:174`, `:192`, `:313`, `:332`, `:336`,
`:428`, `:455`; `screens/notes.rs:30`–`:541`; `screens/settings.rs:64`–`:98`;
`screens/enrolment.rs:11`–`:33`; `lib.rs:495` — plus two literal control sizes, the 36-point
full-width button (`lib.rs:646`) and the 96-point card face (`lib.rs:659`).

*Inference, and it is the practically important one:* **spacing is the capability where the renderer's
ambient mechanism and the app's current practice disagree most.** The renderer would happily carry a
rhythm in `Style::spacing`; the app has instead spelled every gap out. So a spacing decision is not
"set a token and every screen follows" — it is a decision *plus* a migration of sixty call sites, and
until that migration happens a token has no readers. This is the same shape as the problem
ADR-0030 §1 solved for colour, at the moment *before* it was solved.

---

## 6. Icons, and artwork in empty states

The app ships no icon font, no vector sprite, no raster set. What it ships today for an empty state is
a sentence: `crates/app/src/notes.rs:204` — *"Nothing here yet — create a deck, import one, or set up
sync"* — drawn by `body()` at `screens/notes.rs:53`.

Four routes exist. **Their dependency costs were measured, not estimated**: a scratch package
depending only on `egui 0.35.0` resolves to **53 unique crate names**; each route below was resolved
the same way against the live registry and the difference taken. (Names, not packages — where a graph
holds two major versions of the same crate they count once, so these figures are a floor. An
independent resolution counting packages put the largest configuration at 114 against 112 here, which
is the size of that effect.)

**One repo-wide constraint applies to every route that adds a crate from this family**: ADR-0026 §3
and the workspace root pin `egui` and both `eframe` arms at `=0.35.0` exactly, because the host crate
is what pulls in the patched `egui-winit` and *"a floating requirement here is a route to resolving
the family past the version the vendored copy was diffed against."* Any sibling crate added here
inherits that exact pin.

### (a) A glyph font added to the font stack — no new dependency at all

Add the icon face to `font_data` and bind it to a family, exactly as the app already binds its bold
cuts (`crates/app/src/fonts.rs:143-161`). A call site then selects the family and passes the glyph's
code point: `RichText::new("\u{e800}").family(FontFamily::Name("icons".into()))`, or via the same
`FontId` route the app's bidi helper already takes.

There is also a lighter registration call than the one the app uses: `Context::add_font(FontInsert)`
(`context.rs:2061`) adds one face to named families at either end of their fallback chains
(`FontPriority::{Highest, Lowest}`, `fonts.rs:474-484`) without rebuilding the whole definition —
though it still triggers the font-system teardown of §4.5, since the handler sets the font system to
`None` either way (`context.rs:546`).

- **Dependency cost: zero.** No crate is added; the mechanism is the one already in use.
- **Binary cost:** the font file, embedded verbatim and uncompressed. For scale, the four faces the
  app ships total 1,964,992 bytes, and the bundled default set adds roughly 1.4 MB more — so **about
  3.4 MB of the binary is already font data**, against an APK that ADR-0003 §6 records at 5.4 MB. A
  purpose-built icon subset of a few hundred marks is a small fraction of one text face. Worth
  knowing: the bundled default set **already includes an icon font** — `emoji-icon-font.ttf`, 324 KB
  — so this is a shape the stack carries rather than a novelty.
- **Rasterisation cost:** glyphs enter the shared texture atlas **on demand**, one cached entry per
  glyph, size and sub-pixel bin (`epaint-0.35.0/src/text/font.rs:606`, `:634`). The atlas starts 32
  pixels tall and doubles as needed (`epaint-0.35.0/src/texture_atlas.rs`), with a default maximum
  side of 2048 (`text/mod.rs:59`); crossing 80% full triggers the full rebuild of §4.5. So embedding
  a large icon face costs nothing at runtime until a glyph is actually drawn.
- **The constraint:** an icon is then a *character*, so it inherits the text pipeline — one colour per
  run, one size, and no multi-colour artwork. It also has to be registered in the family enumeration
  (`fonts::families()`), and client-stack rule 7's ordering hazard applies: within a family the first
  face carrying a code point wins.
- **The other constraint, and it is this app's specifically:** every user-visible string in this app
  goes through the bidi helper (`AGENTS.md` client-stack rule 1). An icon glyph inserted into a text
  run is a character in that run, so it participates in bidirectional reordering and will need its own
  section in the layout job rather than being concatenated into a neighbouring one.

### (b) The extras crate's image and vector loaders — 14 to 59 crates

`egui_extras 0.35.0` is a separate published package and is **not** a dependency of this workspace
(`crates/app/Cargo.toml` lists `cairn-core`, `cairn-store`, `cairn-export`, `egui`, `unicode-bidi`,
and `eframe` per target — nothing else). Its features, read from the registry's own metadata for
version 0.35.0 exactly:

| feature | what it enables |
|---|---|
| `svg` | `resvg` |
| `svg_text` | `svg` plus the vector rasteriser's text and system-font support |
| `image` | the `image` decoding crate |
| `gif` / `webp` | `image` plus that format |
| `file` (and `default`) | a media-type guesser |
| `http` | an HTTP client |
| `all_loaders` | `file`, `http`, `image`, `svg`, `gif`, `webp` |
| `datepicker`, `syntect` | unrelated widgets |

**Measured resolutions** (unique crates in the dependency graph, against the 53-crate `egui`-only
baseline):

| configuration | total | added |
|---|---|---|
| `egui` alone | 53 | — |
| `+ egui_extras` with `image`, and `image` with the PNG format | **67** | **+14** |
| `+ egui_extras` with `svg` | **80** | **+27** |
| `+ egui_extras` with `svg_text` | **95** | **+42** |
| `+ egui_extras` with `all_loaders` | **112** | **+59** |

What each pulls in concretely:

- **`svg` (+27)** brings a full software vector renderer — a scalable-vector-graphics parser, a
  CPU rasteriser, an XML reader, a CSS-selector matcher, a PNG decoder and a DEFLATE
  decompressor (`resvg`, `usvg`, `tiny-skia`, `tiny-skia-path`, `roxmltree`, `simplecss`, `svgtypes`,
  `png`, `flate2`, `miniz_oxide`, `data-url`, `imagesize`, and support crates). *Notably absent* are
  the font database and text shaper — so with `svg` alone, **`<text>` elements inside a vector file do
  not render**. Icons drawn as filled paths are unaffected; icons that rely on typeset text are not.
- **`svg_text` (+42)** adds exactly those fifteen: a font database that reads the system's installed
  fonts, a second text shaper, a font parser, and the Unicode tables they need. *Inference:* a second
  shaper and a second font database alongside the ones epaint already carries is a duplication this
  app has no other reason to pay for, and system-font access is a platform behaviour the app
  deliberately does not otherwise depend on (it ships its own faces precisely because the platform's
  are not guaranteed).
- **`all_loaders` (+59)** additionally pulls a complete network stack — a TLS implementation, its
  cryptographic backend, a certificate-verification library, a root-certificate bundle, an HTTP
  client and its protocol crate (`rustls`, `ring`, `rustls-webpki`, `webpki-roots`, `ureq`,
  `ureq-proto`, `http`, `httparse`). **This matters beyond size.** ADR-0009 and ADR-0013 §11 put the
  network dependencies in `cairn-sync` deliberately, so that the crates below it need no network. A
  loader umbrella that installs an HTTP client into `cairn-app` to fetch images over the network —
  something this app will never do — puts that stack in the one crate that draws the interface. If
  this route is taken at all, it should be taken by naming `svg` or `image` and never `all_loaders`.

**A trap worth naming, because enabling the obvious feature yields a loader that decodes nothing.**
The extras crate takes the decoding crate with its own default features off, and the loader asks the
decoder at runtime whether a format is enabled — so `egui_extras/image` alone compiles, installs, and
silently declines every file. The extras crate's own documentation says so: *"⚠ You have to configure
both the supported loaders in `egui_extras` **and** the supported image formats in `image` to get any
output!"* Making it work means taking a **second, direct dependency** on the decoding crate with the
formats named — which is why the `image` row in the table above is measured *with* PNG enabled.

The mechanism, so the choice is legible: egui core defines the loader traits — `BytesLoader`
(`egui-0.35.0/src/load.rs:319`), `ImageLoader` (`:390`), `TextureLoader` (`:544`) — and the `Image`
widget (`widgets/image.rs`), and **none of it is feature-gated**; it compiles into every build,
including this app's today. What the extras crate supplies is *implementations*, registered once with
a call that adds them to the context's loader lists. The default set installs a bytes loader (serving
the `include_image!` macro's embedded bytes) and a texture loader, and **no image loader at all**
(`load.rs:608`) — so an `Image` built from encoded bytes has nothing to decode it with and draws an
error placeholder rather than failing to compile.

**Per-target note, and it is narrower than it looks.** ADR-0003 §5's constraint is specifically that
*"`eframe`'s dependency must be split per target. Its default features include `accesskit`, which it
refuses alongside `android-native-activity`."* It names one feature and one crate; it does not
prohibit adding a dependency. `crates/app/Cargo.toml:26-38` implements it — the desktop arm takes the
host crate with defaults, the Android arm with `default-features = false` and exactly
`default_fonts`, `wgpu` and `android-native-activity`. **Both arms therefore render through the same
backend**, so a rendering-capability difference between desktop and handset is not something the
current configuration introduces. And because the split already exists, a decoder *could* be taken on
one arm only — the per-target dependency tables are right there — at the cost of a capability that
exists on the desktop and not on the handset, which is a design question rather than a packaging one.
`cairn-core` takes no user-interface dependency at all (`AGENTS.md`, the workspace's first
easy-to-break-silently rule), so none of these routes may reach it.

### (c) Pre-rasterised textures — no new dependency, but you must supply pixels

`Context::load_texture(name, image, options) -> TextureHandle` (`context.rs:2322-2327`) uploads an
image and hands back a handle. **The handle owns the texture**: cloning it retains, and dropping the
last one frees the texture (`epaint-0.35.0/src/texture_handle.rs:25`, `:31`), so the app must hold it
in its own state for as long as the art is on screen. Drawing it is `ui.image((handle.id(), size))`
(`ui.rs:2033`) or `Painter::image` (`painter.rs:447`) — and because that path is
`ImageSource::Texture`, it **bypasses the loader chain entirely**, so route (c) needs nothing from
route (b).

- **Dependency cost: zero** — this is core API.
- **The catch:** the input is uncompressed pixels — `ColorImage::from_rgba_unmultiplied`
  (`epaint-0.35.0/src/image.rs:113`) or, for a single-channel mask, `from_gray` (`:146`). Producing
  them from a file means either shipping raw colour data in the binary or adding a decoder, which is
  route (b)'s `image` feature. For scale: a 24×24 icon is 2,304 raw bytes, a 48×48 is 9,216, so ten
  marks at 48×48 is about 92 KB uncompressed — against a PNG, which is already compressed on disk.
  *Inference:* an intermediate exists that costs no new dependency — the workspace already carries a
  DEFLATE decompressor transitively, through the archive crate `cairn-export` depends on — so a
  compressed pixel blob could be inflated at startup. Nothing in the repo does this today, and there
  is no asset pipeline to generate such a blob (`scripts/` holds three shell scripts, none of them an
  asset step).
- **It fixes resolution at build time**, which matters more here than usual: the map's *"hit targets
  and density follow touch"* rule spans a desktop at roughly one pixel per point and a handset at
  roughly three, so a texture is either shipped at several sizes or resampled. Routes (a) and (d) are
  resolution-independent; this one is not.

### (d) Painting shapes directly — no new dependency, at the price of writing the drawing

`Painter` (`egui-0.35.0/src/painter.rs`) exposes `line_segment` (`:318`), `line` (`:327`), `hline` /
`vline` (`:332`, `:337`), `circle` / `circle_filled` / `circle_stroke` (`:341`, `:356`, `:370`), `rect`
/ `rect_filled` / `rect_stroke` (`:380`, `:397`, `:406`), `arrow` (`:417`), `image` (`:447`), `text`
(`:469`) and `galley` (`:529`), plus the raw `add` / `extend` / `set` of §2.4. Underneath, `Shape`
(`epaint-0.35.0/src/shapes/shape.rs`) has twelve variants: `Noop`, `Vec`, `Circle`, `Ellipse`,
`LineSegment`, `Path`, `Rect`, `Text`, `Mesh`, `QuadraticBezier`, `CubicBezier`, `Callback`.

**Arbitrary vector art is expressible.** `PathShape` carries a public `closed: bool` alongside its
fill and stroke (`shapes/path_shape.rs:12`), and both Bézier shapes do too
(`shapes/bezier_shape.rs:19`) with flattening helpers that turn a curve into a path
(`:63`, `:299`, `:315`). Closed filled Bézier outlines — which is what a vector icon file compiles
down to — are first-class, and `Shape::Mesh` is the escape hatch below that.

Space is reserved with `Ui::allocate_painter(desired_size, sense) -> (Response, Painter)`
(`ui.rs:1370`), which intersects the clip rectangle so the art cannot paint outside its allocation,
or with `allocate_exact_size` (`:1150`) / `allocate_response` (`:1138`).

Two things this route does *not* cost, worth saying because they are the usual objections:
**hit-testing is solved** — `allocate_painter(size, Sense::click())` returns an ordinary `Response`,
so hover, click and tooltips work exactly as for any widget — and **resolution is handled**, because
shapes are in logical points and the tessellator applies the pixel ratio; `pixels_per_point()`
(`painter.rs:134`) and `round_to_pixel_center` (`:189`) exist for the cases where a one-pixel stroke
must land on a pixel centre.

What it does cost: the coordinates are screen coordinates (`painter.rs:210-212`), so icon geometry has
to be authored normalised and mapped through the allocated rectangle or it will not scale with the
type; the drawing is Rust rather than an asset a designer hands over, with **no diff against the
source art**; and four things a full vector renderer has are missing — there is no fill-rule control
(even-odd versus non-zero), no gradients beyond a single rectangle helper, no clipping paths (only a
rectangular clip), and the convenience polygon constructor is `convex_polygon`
(`shapes/shape.rs:251`), so a concave fill needs decomposing by hand or building as a mesh.

**0.35.0 has a route that makes this much better for icons specifically**, and it is new enough to be
worth naming. Widgets in this version are built from *atoms* — a small layout element that is text, an
image, a nested layout, or a **custom reserved cell**. `Button::new` takes anything convertible into
atoms, so `ui.button((image, "Click me!"))` composes an icon and a label with the button doing the
layout (`egui-0.35.0/src/atomics/atom.rs:17-22`). And `Atom::custom(id, size)`
(`atomics/atom.rs:97`) reserves a cell of a given size inside the widget, whose final rectangle comes
back through `AtomLayoutResponse::rect(id)` (`atomics/atom_layout.rs:722`) — so hand-painted art can
sit inside a button, correctly laid out, aligned and clipped, without re-implementing the button. The
helper `atom_max_height_font_size` (`atomics/atom_ext.rs:63`) sizes such a cell to the text height,
which the source itself notes is *"convenient for icons"*.

*Inference:* for a small, flat, single-colour icon set — which is what a stone-and-slate visual
language would want — route (d) with atoms costs no dependency, no asset pipeline and no atlas, and
the drawing code for a dozen glyph-scale marks is small. Route (a) costs no dependency either and
scales better past a dozen. Route (b) is the only one that buys designer-authored vector files
directly, and its cheapest honest configuration is `svg` at +27 crates with no text support.

**One asymmetry decides a lot of this: route (a) is the only route with an existing, tested code path
in this app.** `fonts::install` already registers faces, `fonts::families()` already enumerates every
family a face must enter, and three tests already hold that enumeration
(`crates/app/src/fonts.rs:150-161` and its test module). Routes (b), (c) and (d) are green field:
**the app has never allocated a texture and has never painted a custom shape.** There is no
`load_texture`, no `TextureHandle`, no `ColorImage`, and no `.painter()` or `allocate_painter` call
anywhere in `crates/app/src/` — the only appearances of `Shape` are two read-only walks over a laid-out
galley inside tests (`screens/review.rs:489`, `screens/notes.rs:581`). Two consequences: adopting a
face costs one entry in an existing list, and an icon face would want a specimen entry too or the
coverage test never checks it; while adopting a painter or a texture means writing the first instance
of a thing this codebase does not yet do.

---

## 7. Ambient roles versus values a screen must name

[ADR-0030 §1](../../adr/0030-the-first-finish-pass-decisions.md) rules that the palette is named in
exactly one place and that every screen reads the *ambient* visuals — asking for a role
(`ui.visuals().text_color()`), never a value. The classification below asks, for each capability,
whether the same discipline is available.

**This is the most decision-relevant output of this note**, because anything in the second column is a
decision with a cost attached rather than a free choice.

### Fully ambient — set once, read implicitly, no screen changes

| Capability | The ambient field | Note |
|---|---|---|
| **Corner radius** | `Visuals::widgets.{noninteractive,inactive,hovered,active,open}.corner_radius`, plus `window_corner_radius` and `menu_corner_radius` | Already state-reactive for free; already in `theme.rs:107-132`. Integer points only. |
| **The type scale** | `Style::text_styles` — the five built-in slots plus arbitrarily many `TextStyle::Name` slots | The app already resolves slots rather than sizes (§4.2), so a scale change reaches every screen without touching one. Registering a name is mandatory or resolution panics. |
| **The font set** | `FontDefinitions` via `Context::set_fonts`, once | Already the app's practice (`fonts::install`). Bold is a family, and must stay one — there is no synthetic bold. |
| **Widget fills, strokes, expansion** | `WidgetVisuals` | Existing practice; listed for completeness. |
| **Touch-sized minimums** | `Spacing::interact_size` | One number expresses the map's *"a 36px button stays 36px"* floor for every button that is not marked small. |

### Ambient in principle, named at the call site today

| Capability | The ambient field | What it would take |
|---|---|---|
| **Spacing and rhythm** | `Spacing::item_spacing`, `button_padding`, `window_margin`, `indent`, and the rest; locally narrowable with `ui.spacing_mut()` inside a `Ui::scope` | The renderer is willing. **The app is not currently reading it**: ~60 literal `ui.add_space(N)` calls and two literal control sizes (§5.3). A spacing token has no readers until those are migrated. The scoping mechanism is sound — a scope is structurally isolated (§5.2) — and costs one style clone per overriding scope per frame. |

### No ambient role exists — every use is a screen naming a value

| Capability | Why | The cost that comes with it |
|---|---|---|
| **Motion** | The animation API is a clock returning an `f32`, not a property system. `Style::animation_time` is read from the *global* style by two functions only, so a screen cannot even override the duration locally (§1.5). No widget animates (§1.7). | Every transition is code in a screen: an `Id` that must be stable across frames or the motion silently vanishes (§1.6), an interpolation the screen writes, and a repaint stream that re-lays-out and re-tessellates the whole application for the duration (§1.4). **There is no way to express "controls fade on hover" once.** If motion is adopted, the honest form is a helper in one module that screens call — the same shape as `theme` and `fonts`, but with a call site per animated thing rather than none. |
| **Elevation and shadow** | `Visuals` carries shadows for **windows and popups only**. There is no shadow on `WidgetVisuals`, so no ordinary widget inherits one (§2.3). | A raised surface is `Frame::new().shadow(…)` wrapped around content at the call site, and the wrapping frame must *also* name an `outer_margin` because the layout reserves no room for a penumbra and it will be clipped at a scroll edge (§2.4). Two named values per elevated surface, and the geometry quantises to whole points. |
| **Icons** | An icon is content. Nothing attaches one to a widget ambiently. | A call site per icon, in every route. What the routes differ on is dependency weight (§6) — 0, +14, +27, +42 or +59 crates — whether the mark is an asset or code, and whether the app is doing something it already does. Only the added-face route reuses an existing, tested path. `Spacing::icon_width` (14.0) and `icon_width_inner` (8.0) exist but govern only the built-in checkbox and expander marks. |
| **Artwork in empty states** | Same as icons, and larger. | Route (c) or (d) of §6, drawn at the call site. Today each empty state is one sentence (§6). |

**Two consequences worth stating out loud.**

First, **motion and elevation are the two capabilities where adopting the design decision and
implementing it are the same act**. For corner radius the decision is five numbers in `theme.rs` and
nothing else moves. For motion there is no `theme.rs` to put it in — a decision like *"state changes
take 150 ms with an ease-out"* is a sentence in an ADR that no code can read, so it becomes a
convention enforced by review rather than by a single naming site. That is precisely the drift
condition ADR-0030 §1 exists to prevent, and it cannot be prevented the same way here.

Second, **`ui.visuals_mut()` is already forbidden by client-stack rule 13** for colour. Several of the
call-site routes above (a per-widget corner override, a locally narrowed spacing scope) work by
mutating a scoped style. Those are not colour, so the rule does not reach them — but the mechanism is
the same one the rule distrusts, and a design that leans on it should say so deliberately rather than
discover it.

---

## 8. What the renderer makes awkward or impossible

**Impossible in 0.35.0:**

- **Synthetic or faux bold.** No weight on a font identity, no bold flag on a text format, no stroke on
  a text shape, no emboldening anywhere, and glyphs are pre-rasterised atlas quads with no outline to
  dilate (§4.3). A heavier face must be shipped, or a variable font adopted.
- **A designer-authored easing curve.** Twenty-one named curves, no cubic-Bézier constructor, and the
  easing parameter is a bare function pointer so a parameterised curve cannot be a closure (§1.3).
- **Fractional corner radii and fractional margins.** Both are integer types by deliberate design
  (`u8` and `i8`), so there is no 2.5-point corner and no 3.5-point margin (§3.1, §5.1). A `Vec2`
  margin additionally collapses to symmetric.
- **A smooth shadow-geometry animation.** The offset, blur and spread are integers, so a geometric
  elevation change steps rather than glides (§2.1). Only the colour is continuous.
- **A true Gaussian shadow.** The blur is a widened linear anti-aliasing feather in gamma space, and it
  is silently clamped to the caster's smaller side (§2.2).
- **A per-screen animation duration.** `Style::animation_time` is read from the global style, so a
  scoped override does nothing (§1.5).
- **Substituting a custom widget-style resolver.** The new per-widget style layer resolves through
  inherent methods on `Style`, not a trait, and the class system it exposes is consulted for exactly
  one class today (§3.4).
- **Several things a full vector renderer has**, if art is painted by hand rather than decoded: no
  fill-rule control, no gradients beyond a single rectangle helper, no clipping paths — the clip is a
  rectangle — and no concave-polygon convenience, since the constructor is `convex_polygon` (§6(d)).

**Awkward — possible, but the cost lands somewhere unexpected:**

- **A shadow inside an ordinary layout.** No layout space is reserved for the penumbra —
  `Frame::total_margin()` omits it and `Shadow::margin()` has no callers anywhere — so it overlaps its
  neighbours and gets clipped at panel and scroll boundaries unless the call site adds a matching
  outer margin by hand (§2.4).
- **Animating anything whose identity is not stable.** The animation store is keyed by widget `Id`,
  the first frame for a new `Id` snaps to the target, and nothing is ever collected. An `Id` derived
  from a list index or an editable string produces no motion at all, with nothing failing (§1.6).
- **Any motion at all, on a battery.** One in-flight transition repaints the whole viewport at the
  display's rate, and the renderer deliberately does not cache tessellation between frames, so each of
  those ~12 frames re-lays-out and re-tessellates everything on screen (§1.4).
- **Hover, press and focus polish.** Nothing under `widgets/` animates; state visuals are swapped
  instantaneously. Every bit of that polish is the app's to write (§1.7).
- **Changing fonts after startup.** It byte-compares every embedded face, then rebuilds the texture
  atlas and empties the layout cache (§4.5). This is why the app installs once, on the first frame.
  A future light palette inherits a related cost: the stock light and dark visuals specify different
  colour transfer functions, so a theme switch forces the same rebuild.
- **A named text style or font family that was never registered.** Both panic at use rather than
  falling back (§4.1).
- **Reaching the `open` widget state.** The classic selector never chooses it and the new
  `WidgetState` enum cannot represent it (§3.4); a widget that wants it must ask for it explicitly.
- **Anything below the `Painter`.** Hand-painted art is screen coordinates, hand-written hit testing,
  and manual pixel-ratio rounding — mitigated for widget-sized marks by the reserved-cell atom of §6(d),
  not eliminated.

---

## Confidence, and what this note does not answer

**High confidence, read directly from the pinned sources:** every type definition, field, default
value, signature and line citation in §1–§6, and the dependency-count measurements in §6, which were
produced by resolving scratch packages with the same toolchain and lockfile constraints this workspace
uses.

**Inference, marked as such where it appears:** the per-frame cost arithmetic in §1.4 and §2.2 (derived
from the reserve-counts and the no-caching comment, not measured on a device); the leak characterisation
in §1.6; the visual characterisation of the linear-ramp blur in §2.2; the reading of the new widget-style
layer as an in-progress migration in §3.4; and the whole of §7's classification, which is a judgement
about *this app's* call sites rather than a fact about the renderer.

**Not answered here, deliberately:**

- **Nothing was run.** No frame was rendered and no timing was taken. The frame-cost figures are
  arithmetic over the tessellator's own reserve counts, not measurements — and per `AGENTS.md` rule 9,
  any Android claim would need the handset anyway. If a motion decision turns on whether twelve frames
  of full re-tessellation is acceptable on the Pixel, **that is a measurement someone still owes**, and
  it is a `/prototype` or a handset ticket, not a reading.
- **What the design should choose.** This note establishes that six of seven capabilities are
  expressible and at what cost. Which of them Cairn adopts is the map's business.
- **The 12.5px body-size discrepancy** in ADR-0030 §3 (§4.1) is recorded, not resolved; where that
  figure came from was not traced.
