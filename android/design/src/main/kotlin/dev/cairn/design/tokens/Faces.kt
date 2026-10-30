package dev.cairn.design.tokens

/**
 * The bundled faces, in the order each family reaches them (ADR-0038 §1). Mirrors
 * `crates/app/src/fonts.rs`: the guard reads these lists, opens each file under
 * `src/main/assets/fonts` and compares its bytes with the face egui registers at that position.
 *
 * Comma-separated file names, because a `const val` cannot hold a list. Cairn Icons is last in
 * every family: it is private use, so it shadows nothing and nothing shadows it. Emoji is left to
 * the platform's own fallback.
 */
object Faces {
    const val PROPORTIONAL =
        "Ubuntu-Light.ttf,NotoSansArabic-Regular.ttf,DejaVuSans.ttf,CairnIcons-Regular.ttf"
    const val MONOSPACE =
        "Hack-Regular.ttf,Ubuntu-Light.ttf,NotoSansArabic-Regular.ttf,DejaVuSans.ttf,CairnIcons-Regular.ttf"
    const val BOLD = "NotoSansArabic-Bold.ttf,DejaVuSans-Bold.ttf,CairnIcons-Regular.ttf"

    /** The mark, one private-use code point (`fonts::MARK`). */
    const val MARK = 0xE000
}
