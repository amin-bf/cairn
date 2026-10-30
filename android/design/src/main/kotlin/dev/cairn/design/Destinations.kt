package dev.cairn.design

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.ui.platform.LocalDensity
import dev.cairn.design.tokens.Frame
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Spacing

/** One word of the destinations row: its label, whether it is the page showing, and its test tag. */
data class Destination(val label: String, val selected: Boolean, val tag: String, val onClick: () -> Unit)

/**
 * The destinations row (ADR-0048 §1): the places the app has, as words in a row, the one showing
 * marked on the selection colour. Pinned by the caller while the keyboard is down.
 */
@Composable
fun CairnDestinationsRow(destinations: List<Destination>, modifier: Modifier = Modifier) {
    val colors = Cairn.colors
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(Spacing.UNIT.dp), verticalAlignment = Alignment.CenterVertically) {
        for (destination in destinations) {
            Box(
                Modifier
                    .defaultMinSize(minHeight = Controls.HEIGHT.dp)
                    .clip(RoundedCornerShape(Controls.WIDGET_RADIUS.dp))
                    .background(if (destination.selected) colors.selection else colors.page)
                    .semantics { selected = destination.selected }
                    .clickable(onClick = destination.onClick)
                    .padding(horizontal = (Spacing.UNIT * 2).dp)
                    .testTag(destination.tag),
                contentAlignment = Alignment.Center,
            ) {
                CairnText(destination.label, color = if (destination.selected) colors.body else colors.onControl)
            }
        }
    }
}

/** Whether the soft keyboard is up, read from its insets. */
@Composable
fun rememberKeyboardUp(): Boolean = WindowInsets.ime.getBottom(LocalDensity.current) > 0

/**
 * The shell around every destination (ADR-0048 §1): the destinations row pinned at the top while the
 * keyboard is down and absent while it is up, so an editor's first screen is the form's. The shell
 * takes the system insets, so the page inside it asks for none twice.
 */
@Composable
fun CairnShell(
    destinations: List<Destination>,
    modifier: Modifier = Modifier,
    keyboardUp: Boolean = rememberKeyboardUp(),
    content: @Composable () -> Unit,
) {
    Column(modifier.fillMaxSize().background(Cairn.colors.page).windowInsetsPadding(WindowInsets.safeDrawing)) {
        if (!keyboardUp) {
            Box(Modifier.fillMaxWidth().padding(horizontal = Frame.PAGE_MARGIN.dp), contentAlignment = Alignment.TopCenter) {
                CairnDestinationsRow(destinations, Modifier.widthIn(max = Frame.MEASURE.dp).fillMaxWidth().testTag("destinations"))
            }
        }
        Box(Modifier.weight(1f)) { content() }
    }
}
