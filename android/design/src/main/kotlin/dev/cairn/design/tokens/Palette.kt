package dev.cairn.design.tokens

/**
 * The two palettes (ADR-0030, ADR-0036), one role per name, as ARGB. Mirrors `crates/app/src/theme.rs`.
 *
 * The roles are what the Rust side already reads from egui's `Visuals` and its own `card_fill`,
 * `card_divider` and `primary_fill`, so the guard compares each value with the one the desktop
 * draws and not with a second copy of a constant. Colour is named here and nowhere else in the
 * Android client (AGENTS.md, client-stack rule 13): a screen asks [dev.cairn.design.CairnColors]
 * for a role.
 *
 * [WEAK] is the weak text colour as drawn, already composited over [PAGE]: dark derives it from the
 * body colour at the visuals' `weak_text_alpha`, light names it.
 */
object DarkPalette {
    const val PAGE = 0xFF1A1E21L
    const val POPUP = 0xFF22282BL
    const val POPUP_STROKE = 0xFF282E33L
    const val FIELD = 0xFF15191BL
    const val CONTROL = 0xFF21262AL
    const val CARD = 0xFF0F1214L
    const val CARD_DIVIDER = 0xFF282E33L
    const val SEPARATOR = 0xFF282E33L
    const val BODY = 0xFFE2E6E6L
    const val WEAK = 0xFF929697L
    const val ON_CONTROL = 0xFFB9C2C3L
    const val PRIMARY = 0xFF2C3237L
    const val PRIMARY_STROKE = 0xFF333B40L
    const val HOVER = 0xFF363D43L
    const val HOVER_STROKE = 0xFF8B979BL
    const val PRESSED = 0xFF282E33L
    const val SELECTION = 0xFF2A4453L
    const val SELECTION_STROKE = 0xFFCFE3ECL
    const val LINK = 0xFF6F93A8L
    const val WARN = 0xFFC2A37AL
    const val ERROR = 0xFFB57E79L
}

object LightPalette {
    const val PAGE = 0xFFDEE2E3L
    const val POPUP = 0xFFEAEFF0L
    const val POPUP_STROKE = 0xFFCED2D3L
    const val FIELD = 0xFFD2D6D7L
    const val CONTROL = 0xFFD9DDDDL
    const val CARD = 0xFFC4C8C9L
    const val CARD_DIVIDER = 0xFFA7ABACL
    const val SEPARATOR = 0xFFCED2D3L
    const val BODY = 0xFF171B1DL
    const val WEAK = 0xFF535759L
    const val ON_CONTROL = 0xFF333739L
    const val PRIMARY = 0xFFA2A6A7L
    const val PRIMARY_STROKE = 0xFF979B9DL
    const val HOVER = 0xFFB9BDBEL
    const val HOVER_STROKE = 0xFF565A5CL
    const val PRESSED = 0xFFCED2D3L
    const val SELECTION = 0xFFCFE3ECL
    const val SELECTION_STROKE = 0xFF485F6DL
    const val LINK = 0xFF485F6DL
    const val WARN = 0xFF534634L
    const val ERROR = 0xFF7A5551L
}
