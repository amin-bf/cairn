package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.cairn.design.tokens.Surface
import dev.cairn.design.tokens.Typography

/**
 * The well (ADR-0033 §1's material, ADR-0043 §2): a filled, stroked, rounded surface that holds a
 * card's text and nothing the person presses. It is the one place a list draws a card, so it says
 * *this is one thing*; the controls that act on it sit outside it, below, and stay quieter than it.
 */
@Composable
fun CairnWell(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    val colors = Cairn.colors
    val shape = RoundedCornerShape(Surface.RADIUS.dp)
    Column(
        modifier
            .fillMaxWidth()
            .clip(shape)
            .background(colors.card)
            .border(BorderStroke(1.dp, colors.separator), shape)
            .padding(gap(2)),
        content = content,
    )
}

/**
 * A card's [runs] as the card draws them — each glyph in the face that has it, each run in its own
 * style — at the body tier, set in the direction of the text itself. For a well, which holds a prompt.
 */
@Composable
fun CairnRuns(runs: List<CardRun>, modifier: Modifier = Modifier) {
    val faces = LocalFaces.current
    val text = remember(runs, faces) { faces.annotate(runs) }
    Box(modifier.fillMaxWidth(), contentAlignment = Alignment.TopStart) {
        BasicText(
            text,
            style = TextStyle(
                color = Cairn.colors.body,
                fontSize = Typography.BODY.sp,
                textDirection = TextDirection.Content,
            ),
        )
    }
}
