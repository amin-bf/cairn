package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Spacing

/**
 * A control: the ordinary one by default, the primary when [primary] (ADR-0033). The primary is the
 * one that is pressed to go on; it is a heavier fill, not a different shape.
 */
@Composable
fun CairnControl(label: String, onClick: () -> Unit, modifier: Modifier = Modifier, primary: Boolean = false) {
    val colors = Cairn.colors
    val shape = RoundedCornerShape(Controls.WIDGET_RADIUS.dp)
    Box(
        modifier
            .defaultMinSize(minHeight = Controls.HEIGHT.dp)
            .clip(shape)
            .background(if (primary) colors.primary else colors.control)
            .border(BorderStroke(1.dp, if (primary) colors.primaryStroke else colors.separator), shape)
            .clickable(onClick = onClick)
            .padding(horizontal = (Spacing.UNIT * 2).dp),
        contentAlignment = Alignment.Center,
    ) {
        CairnText(label, color = colors.onControl)
    }
}
