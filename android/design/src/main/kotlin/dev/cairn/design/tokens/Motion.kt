package dev.cairn.design.tokens

/**
 * Motion (ADR-0037 §2). Mirrors `crates/app/src/motion.rs`.
 *
 * [DURATION] is in seconds, as in Rust. The easing is `cubic_out`, `1 - (1 - t)^3`: a function
 * cannot be compared as text, so the guard compares [EASING_AT_HALF] with Rust's `EASING(0.5)` and
 * a unit test here checks [easing] against it.
 */
object Motion {
    const val DURATION = 0.24f
    const val EASING_AT_HALF = 0.875f

    fun easing(t: Float): Float {
        val rest = 1f - t
        return 1f - rest * rest * rest
    }
}
