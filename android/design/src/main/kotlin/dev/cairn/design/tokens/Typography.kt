package dev.cairn.design.tokens

/**
 * The type scale (ADR-0032). Mirrors `crates/app/src/typography.rs` number for number: Rust's
 * logical pixels are the same quantity as `sp` here, so nothing is converted.
 *
 * `cairn-app`'s `kotlin_tokens` test reads this file and fails on any difference or missing token.
 * This is the only place in the Android client that names a font size (AGENTS.md, client-stack
 * rule 17): ask for a tier through [dev.cairn.design.CairnText], never for a number.
 */
object Typography {
    const val DISPLAY = 40f
    const val HEADING = 20f
    const val BODY = 15f
    const val SMALL = 12f

    /** The mark, drawn large. A font size and not a fifth tier (ADR-0038 §3). */
    const val MARK = 104f
}
