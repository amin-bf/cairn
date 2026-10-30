package dev.cairn.design

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DirectionTest {
    @Test
    fun persian_text_sets_right_to_left() {
        assertTrue(isRightToLeft("سلام"))
    }

    @Test
    fun the_first_strong_character_decides_not_the_majority() {
        assertTrue(isRightToLeft("1) \"سلام hello world\""))
        assertFalse(isRightToLeft("le mot سلام"))
    }

    @Test
    fun empty_text_and_text_with_no_letter_set_left_to_right() {
        assertFalse(isRightToLeft(""))
        assertFalse(isRightToLeft("123 …"))
    }
}
