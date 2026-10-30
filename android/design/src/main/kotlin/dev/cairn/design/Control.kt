package dev.cairn.design

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls

/**
 * A text action: a label with no surface, in the link colour at the small tier (ADR-0034 §2). The one
 * frameless control, for a short set of alternatives sitting beside a primary. It keeps the 36 dp
 * height every control has, so it is as easy to hit as the framed ones.
 */
@Composable
fun CairnTextAction(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Box(
        modifier
            .defaultMinSize(minWidth = Controls.HEIGHT.dp, minHeight = Controls.HEIGHT.dp)
            .clickable(onClick = onClick)
            .padding(horizontal = gap(1)),
        contentAlignment = Alignment.Center,
    ) {
        CairnText(label, tier = Tier.Small, color = Cairn.colors.link)
    }
}

/**
 * A control: the ordinary one by default, the primary when [primary] (ADR-0033). The primary is the
 * one that is pressed to go on; it is a heavier fill, not a different shape.
 */
@Composable
fun CairnControl(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    aside: String? = null,
    enabled: Boolean = true,
) {
    val colors = Cairn.colors
    Box(
        modifier
            .controlSurface(
                if (primary) colors.primary else colors.control,
                if (primary) colors.primaryStroke else colors.separator,
            )
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = gap(2)),
        contentAlignment = Alignment.Center,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            // A control that cannot be pressed speaks in the weak ink, as a hint rather than a command.
            CairnText(label, color = if (enabled) colors.onControl else colors.weak)
            // An aside is a second, quieter fact about the same choice: small and dimmed (ADR-0034 §1).
            if (aside != null) {
                Spacer(Modifier.width(gap(1)))
                CairnText(aside, tier = Tier.Small, color = colors.weak)
            }
        }
    }
}
