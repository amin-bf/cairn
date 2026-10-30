package dev.cairn.design

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Frame

/**
 * The page: the page colour edge to edge, and one centred column inside it, a margin from each edge
 * and no wider than the measure (ADR-0031). A screen asks for the frame and never computes a width.
 */
@Composable
fun CairnPage(modifier: Modifier = Modifier, content: @Composable BoxScope.() -> Unit) {
    Box(
        modifier.fillMaxSize().background(Cairn.colors.page).padding(horizontal = Frame.PAGE_MARGIN.dp),
        contentAlignment = Alignment.TopCenter,
    ) {
        Box(Modifier.widthIn(max = Frame.MEASURE.dp).fillMaxWidth(), content = content)
    }
}
