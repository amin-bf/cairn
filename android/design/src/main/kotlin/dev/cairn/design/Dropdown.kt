package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupPositionProvider
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.window.PopupProperties
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Surface

/**
 * A dropdown: the [selected] label in a control-sized box, and on a tap the [options] in a popup
 * on the popup material (ADR-0037 §2) directly under it. [onSelect] is told the option's place. The
 * options are tagged `"<tag>-option-<place>"` so a test can reach one.
 */
@Composable
fun CairnDropdown(
    selected: String,
    options: List<String>,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
    tag: String = "dropdown",
) {
    val colors = Cairn.colors
    var open by rememberSaveable { mutableStateOf(false) }
    Box(modifier.fillMaxWidth()) {
        Box(
            Modifier
                .fillMaxWidth()
                .controlSurface(colors.control, colors.separator)
                .clickable { open = true }
                .padding(horizontal = gap(2))
                .testTag(tag),
            contentAlignment = Alignment.CenterStart,
        ) {
            CairnText(selected, color = colors.onControl)
        }
        if (open) {
            Popup(
                popupPositionProvider = Below,
                onDismissRequest = { open = false },
                properties = PopupProperties(focusable = true),
            ) {
                val card = RoundedCornerShape(Surface.RADIUS.dp)
                Column(
                    Modifier
                        .fillMaxWidth()
                        .clip(card)
                        .background(colors.popup)
                        .border(BorderStroke(1.dp, colors.popupStroke), card),
                ) {
                    options.forEachIndexed { place, label ->
                        Box(
                            Modifier
                                .fillMaxWidth()
                                .defaultMinSize(minHeight = Controls.HEIGHT.dp)
                                .clickable {
                                    open = false
                                    onSelect(place)
                                }
                                .padding(horizontal = gap(2))
                                .testTag("$tag-option-$place"),
                            contentAlignment = Alignment.CenterStart,
                        ) { CairnText(label) }
                    }
                }
            }
        }
    }
}

/** The popup sits directly under its anchor, as wide as it. */
private object Below : PopupPositionProvider {
    override fun calculatePosition(
        anchorBounds: IntRect,
        windowSize: IntSize,
        layoutDirection: LayoutDirection,
        popupContentSize: IntSize,
    ) = IntOffset(anchorBounds.left, anchorBounds.bottom)
}
