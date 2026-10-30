package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Spacing

/** A stated gap of [units] rhythm units (ADR-0032): an integer, so a half-step will not compile. */
fun gap(units: Int): Dp = (Spacing.UNIT * units).dp

/**
 * The surface every control-sized box shares — a control, a list row, a dropdown's anchor: the 36 dp
 * minimum height, the widget corner, a [fill] inside a one-pixel [stroke]. One definition, so the
 * corner, the height and the line cannot drift apart one box at a time.
 */
fun Modifier.controlSurface(fill: Color, stroke: Color): Modifier {
    val shape = RoundedCornerShape(Controls.WIDGET_RADIUS.dp)
    return this
        .defaultMinSize(minHeight = Controls.HEIGHT.dp)
        .clip(shape)
        .background(fill)
        .border(BorderStroke(1.dp, stroke), shape)
}
