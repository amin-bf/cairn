package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Surface

/**
 * A dormant entry (ADR-0018 §2): the empty place a card used to be. It takes the card's footprint and
 * corner with no fill — an outline where a card is a well — and carries its [line] at the aside's
 * weight.
 */
@Composable
fun CairnOutlineEntry(line: String, modifier: Modifier = Modifier) {
    val colors = Cairn.colors
    Box(
        modifier
            .fillMaxWidth()
            .border(BorderStroke(1.dp, colors.separator), RoundedCornerShape(Surface.RADIUS.dp))
            .padding(gap(2))
            .semantics(mergeDescendants = true) {},
    ) {
        CairnText(line, tier = Tier.Small, color = colors.weak)
    }
}

/**
 * The ambient warning's boundary (ADR-0012 §5, ADR-0025 §4): [content] inside a left rule at the
 * separator rung — no fill, no new value, so it reads as a statement about the form and not a control.
 */
@Composable
fun CairnLeftRule(modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    Row(modifier.fillMaxWidth().height(IntrinsicSize.Min)) {
        Box(Modifier.width(1.dp).fillMaxHeight().background(Cairn.colors.separator))
        Column(Modifier.padding(start = gap(2))) { content() }
    }
}
