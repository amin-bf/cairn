package dev.cairn.design.tokens

/**
 * Controls. Mirrors `crates/app/src/controls.rs`; values are `dp`.
 *
 * [WIDGET_RADIUS] is egui's `widgets.inactive.corner_radius` in the Cairn visuals, which the Rust
 * side never names as a constant of its own; the guard reads it from the visuals.
 */
object Controls {
    const val HEIGHT = 36f
    const val TARGET_INK = 131
    const val WIDGET_RADIUS = 2f
}
