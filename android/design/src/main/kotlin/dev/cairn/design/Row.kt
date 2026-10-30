package dev.cairn.design

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Controls

/** The pictures a row's action column can hold (ADR-0039 §1): the 24 dp grid, 1.5 stroke, round caps. */
enum class RowIcon(internal val path: String) {
    /** `crates/app/icons/move.svg`: a vertical double-headed arrow. */
    Move("M12 4.75v14.5M12 4.75l-3.5 3.5M12 4.75l3.5 3.5M12 19.25l-3.5-3.5M12 19.25l3.5-3.5"),

    /** `crates/app/icons/delete.svg`: a bin. */
    Delete("M5 7h14M9.5 7V4.5h5V7M7.5 7l.9 12.5h7.2L16.5 7"),
}

/**
 * One icon action in a row's right-hand column. It stands as a picture alone, and carries its word
 * as [description] for the screen reader, which is where a pointer would hover (ADR-0039 §1).
 */
class RowAction(
    val icon: RowIcon,
    val description: String,
    val tag: String,
    val onClick: () -> Unit,
)

private val bandShape = RoundedCornerShape(Controls.WIDGET_RADIUS.dp)

/** Draws [icon] in [color] on its 24 dp grid, centred in whatever it is given. */
@Composable
private fun RowGlyph(icon: RowIcon, color: Color, modifier: Modifier = Modifier) {
    val path = remember(icon) { PathParser().parsePathString(icon.path).toPath() }
    Canvas(modifier.size(24.dp)) {
        val scale = size.width / 24f
        scale(scale, pivot = Offset.Zero) {
            drawPath(path, color, style = Stroke(1.5f, cap = StrokeCap.Round, join = StrokeJoin.Round))
        }
    }
}

/**
 * A list row (ADR-0039): one band, the [title] on at most two lines, and — only when the list has a
 * reason to say it — a quiet [caption] under it. The title takes the band's width and sets in the
 * direction of its own text; the [actions] are a right-aligned column of 36 dp squares that stays on
 * the right in both directions, because furniture is not content (ADR-0039 §4). [onClick] opens the row.
 */
@Composable
fun CairnListRow(
    title: String,
    caption: String?,
    modifier: Modifier = Modifier,
    actions: List<RowAction> = emptyList(),
    onClick: (() -> Unit)? = null,
) {
    val colors = Cairn.colors
    CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Ltr) {
        Row(
            modifier
                .fillMaxWidth()
                .controlSurface(colors.control, colors.separator),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(
                Modifier
                    .weight(1f)
                    .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
                    .padding(horizontal = gap(2), vertical = gap(1)),
            ) {
                CairnText(title, Modifier.fillMaxWidth(), color = colors.onControl, maxLines = 2)
                if (caption != null) {
                    CairnText(caption, Modifier.fillMaxWidth(), tier = Tier.Small, color = colors.weak, maxLines = 1)
                }
            }
            for (action in actions) {
                Box(
                    Modifier
                        .size(Controls.HEIGHT.dp)
                        .testTag(action.tag)
                        .semantics { contentDescription = action.description }
                        .clickable(role = Role.Button, onClick = action.onClick),
                    contentAlignment = Alignment.Center,
                ) {
                    RowGlyph(action.icon, colors.onControl)
                }
            }
        }
    }
}

/**
 * A row that offers nothing (ADR-0039 §7): the same band with no actions, for a note a held note is
 * being placed among. [held] draws it on the popup surface — *temporarily on top*, a note picked up
 * and not yet put down.
 */
@Composable
fun CairnInertRow(title: String, modifier: Modifier = Modifier, held: Boolean = false) {
    val colors = Cairn.colors
    Column(
        modifier
            .semantics(mergeDescendants = true) {}
            .fillMaxWidth()
            .controlSurface(if (held) colors.popup else colors.control, if (held) colors.popupStroke else colors.separator)
            .padding(horizontal = gap(2), vertical = gap(1)),
    ) {
        CairnText(title, Modifier.fillMaxWidth(), color = colors.onControl, maxLines = 2)
    }
}

/**
 * A placement target (ADR-0039 §7): *where*, not *what*, so its fill and word are drawn at
 * [Controls.TARGET_INK] of full while the hit area stays a whole control tall.
 */
@Composable
fun CairnQuietTarget(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val colors = Cairn.colors
    val ink = Controls.TARGET_INK / 255f
    Box(
        modifier
            .fillMaxWidth()
            .height(Controls.HEIGHT.dp)
            .clip(bandShape)
            .background(colors.control.copy(alpha = colors.control.alpha * ink))
            .clickable(role = Role.Button, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        CairnText(label, color = colors.body.copy(alpha = colors.body.alpha * ink))
    }
}

/**
 * A row asking before it destroys (ADR-0048 §4, in the shape ADR-0039 §6 gives a deck): the [question]
 * in place of the note, then the confirming and the keeping control, stacked so neither crushes the other.
 */
@Composable
fun CairnConfirmRow(
    question: String,
    confirm: String,
    keep: String,
    onConfirm: () -> Unit,
    onKeep: () -> Unit,
    modifier: Modifier = Modifier,
    questionModifier: Modifier = Modifier,
    confirmModifier: Modifier = Modifier,
    keepModifier: Modifier = Modifier,
) {
    val colors = Cairn.colors
    Column(
        modifier
            .fillMaxWidth()
            .controlSurface(colors.control, colors.separator)
            .padding(horizontal = gap(2), vertical = gap(1)),
        verticalArrangement = Arrangement.spacedBy(gap(1)),
    ) {
        CairnText(question, Modifier.fillMaxWidth().then(questionModifier), color = colors.onControl)
        CairnControl(confirm, onClick = onConfirm, modifier = confirmModifier)
        CairnControl(keep, onClick = onKeep, modifier = keepModifier)
    }
}

/** The hairline that ends the chrome and starts the list (ADR-0039 §2), on the divider colour. */
@Composable
fun CairnRule(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(1.dp).background(Cairn.colors.cardDivider))
}
