package dev.cairn.design

import org.junit.Assert.assertEquals
import org.junit.Test

class BlankTest {
    @Test
    fun a_selection_is_wrapped_as_the_numbered_blank_with_the_caret_after_it() {
        val (text, caret) = wrapAsBlank("Le chat mange", 3, 7, 1)
        assertEquals("Le {{1::chat}} mange", text)
        assertEquals("Le {{1::chat}}".length, caret)
    }

    @Test
    fun a_backwards_selection_wraps_the_same_stretch() {
        assertEquals("Le {{2::chat}} mange", wrapAsBlank("Le chat mange", 7, 3, 2).first)
    }

    @Test
    fun persian_and_a_zero_width_joiner_are_wrapped_whole() {
        val text = "سلام ‌دنیا"
        val (wrapped, _) = wrapAsBlank(text, 0, 4, 3)
        assertEquals("{{3::سلام}} ‌دنیا", wrapped)
    }
}
