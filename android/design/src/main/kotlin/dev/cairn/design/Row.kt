package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Spacing

/**
 * A list row (ADR-0039): one band, the [title] on at most two lines, and — only when the list has a
 * reason to say it — a quiet [caption] under it. The title takes the band's width and sets in the
 * direction of its own text. [onClick] opens the row.
 */
@Composable
fun CairnListRow(
    title: String,
    caption: String?,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
) {
    val colors = Cairn.colors
    val shape = RoundedCornerShape(Controls.WIDGET_RADIUS.dp)
    Column(
        modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = Controls.HEIGHT.dp)
            .clip(shape)
            .background(colors.control)
            .border(BorderStroke(1.dp, colors.separator), shape)
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = (Spacing.UNIT * 2).dp, vertical = Spacing.UNIT.dp),
    ) {
        CairnText(title, Modifier.fillMaxWidth(), color = colors.onControl, maxLines = 2)
        if (caption != null) {
            CairnText(caption, Modifier.fillMaxWidth(), tier = Tier.Small, color = colors.weak, maxLines = 1)
        }
    }
}

/** The hairline that ends the chrome and starts the list (ADR-0039 §2), on the divider colour. */
@Composable
fun CairnRule(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(1.dp).background(Cairn.colors.cardDivider))
}
