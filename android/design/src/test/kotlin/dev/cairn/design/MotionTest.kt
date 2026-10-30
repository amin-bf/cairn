package dev.cairn.design

import dev.cairn.design.tokens.Motion
import org.junit.Assert.assertEquals
import org.junit.Test

class MotionTest {
    @Test
    fun the_easing_is_cubic_out_and_agrees_with_the_value_the_rust_guard_pins() {
        assertEquals(Motion.EASING_AT_HALF, Motion.easing(0.5f), 1e-6f)
        assertEquals(0f, Motion.easing(0f), 0f)
        assertEquals(1f, Motion.easing(1f), 0f)
    }
}
