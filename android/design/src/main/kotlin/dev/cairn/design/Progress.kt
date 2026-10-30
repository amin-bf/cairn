package dev.cairn.design

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.BiasAlignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls

/**
 * A bar of work done: the track on the control rung and the primary fill across [fraction] of it.
 * Without a [fraction] it is the indeterminate form, a short segment sliding along the track — for
 * work that has no count yet (ADR-0014 §4's *Preparing*). It takes the control's corner and the full
 * width it is given.
 */
@Composable
fun CairnProgress(fraction: Float?, modifier: Modifier = Modifier) {
    val colors = Cairn.colors
    Box(
        modifier
            .fillMaxWidth()
            .height(gap(1))
            .clip(RoundedCornerShape(Controls.WIDGET_RADIUS.dp))
            .background(colors.control),
    ) {
        if (fraction != null) {
            Box(Modifier.fillMaxWidth(fraction.coerceIn(0f, 1f)).fillMaxHeight().background(colors.primary))
        } else {
            val slide by rememberInfiniteTransition(label = "preparing").animateFloat(
                initialValue = -1f,
                targetValue = 1f,
                animationSpec = infiniteRepeatable(tween(1_200, easing = LinearEasing), RepeatMode.Reverse),
                label = "slide",
            )
            // A segment of a third of the track, carried from one end to the other and back.
            Box(Modifier.fillMaxWidth().fillMaxHeight(), contentAlignment = BiasAlignment(slide, 0f)) {
                Box(Modifier.fillMaxWidth(0.3f).fillMaxHeight().background(colors.primary))
            }
        }
    }
}
