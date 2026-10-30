package dev.cairn.design

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.layoutId
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Frame

/**
 * The page: the page colour edge to edge, and one centred column inside it, a margin from each edge
 * and no wider than the measure (ADR-0031). A screen asks for the frame and never computes a width.
 * The column keeps clear of the system bars and the keyboard, so "the page's bottom" is the bottom
 * of the room the person can actually use.
 */
@Composable
fun CairnPage(modifier: Modifier = Modifier, content: @Composable BoxScope.() -> Unit) {
    Box(
        modifier
            .fillMaxSize()
            .background(Cairn.colors.page)
            .windowInsetsPadding(WindowInsets.safeDrawing)
            .padding(horizontal = Frame.PAGE_MARGIN.dp),
        contentAlignment = Alignment.TopCenter,
    ) {
        Box(Modifier.widthIn(max = Frame.MEASURE.dp).fillMaxSize(), content = content)
    }
}

private enum class Slot { Top, Reach }

/**
 * A column of [top] and, below it, the [reach] block — the controls pressed on every card — placed so
 * the block's bottom edge lands [Frame.REACH_LINE] above the bottom of a page [room] tall, the
 * slack falling between the two (ADR-0035 §2). Where [room] leaves none, [reach] follows [top] at
 * [floor]: one expression for the handset and for anything shorter, with no breakpoint. The column
 * is as tall as the page at least, so it scrolls only when its content is taller.
 */
@Composable
fun ReachColumn(
    room: Dp,
    floor: Dp,
    modifier: Modifier = Modifier,
    top: @Composable () -> Unit,
    reach: @Composable () -> Unit,
) {
    Layout(
        content = {
            Box(Modifier.layoutId(Slot.Top)) { top() }
            Box(Modifier.layoutId(Slot.Reach)) { reach() }
        },
        modifier = modifier,
    ) { measurables, constraints ->
        val loose = Constraints(maxWidth = constraints.maxWidth)
        val upper = measurables.first { it.layoutId == Slot.Top }.measure(loose)
        val lower = measurables.first { it.layoutId == Slot.Reach }.measure(Constraints.fixedWidth(constraints.maxWidth))
        val line = Frame.REACH_LINE.dp.roundToPx()
        val y = maxOf(upper.height + floor.roundToPx(), room.roundToPx() - line - lower.height)
        layout(constraints.maxWidth, y + lower.height + line) {
            upper.place(0, 0)
            lower.place(0, y)
        }
    }
}
