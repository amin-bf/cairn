package dev.cairn.design

import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.sp
import dev.cairn.design.tokens.Faces
import dev.cairn.design.tokens.Typography

/** The four tiers of the scale (ADR-0032). */
enum class Tier(internal val size: Float) {
    Display(Typography.DISPLAY),
    Heading(Typography.HEADING),
    Body(Typography.BODY),
    Small(Typography.SMALL),
}

/**
 * Text at a tier, in a family, with every glyph reaching the face that has it. The direction follows
 * the content, so a Persian sentence sets right to left and a French one left to right.
 */
@Composable
fun CairnText(
    text: String,
    modifier: Modifier = Modifier,
    tier: Tier = Tier.Body,
    family: Family = Family.Proportional,
    color: Color = Cairn.colors.body,
    maxLines: Int = Int.MAX_VALUE,
) {
    val faces = LocalFaces.current
    val resolved = remember(text, family, faces) { faces.resolve(text, family) }
    BasicText(
        resolved,
        modifier,
        maxLines = maxLines,
        overflow = TextOverflow.Ellipsis,
        style = TextStyle(
            color = color,
            fontSize = tier.size.sp,
            textDirection = TextDirection.Content,
        ),
    )
}

/** The mark, four stones: one private-use character drawn at its own size (ADR-0038 §3). */
@Composable
fun Mark(modifier: Modifier = Modifier, color: Color = Cairn.colors.body) {
    val faces = LocalFaces.current
    val resolved = remember(faces) { faces.resolve(String(Character.toChars(Faces.MARK)), Family.Proportional) }
    BasicText(
        resolved,
        modifier,
        style = TextStyle(color = color, fontSize = Typography.MARK.sp),
    )
}
