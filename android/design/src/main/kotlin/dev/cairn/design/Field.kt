package dev.cairn.design

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Spacing
import dev.cairn.design.tokens.Typography
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth

/** A field's label: small and quiet, set over the field it names. */
@Composable
fun CairnFieldLabel(text: String, modifier: Modifier = Modifier) {
    CairnText(text, modifier, tier = Tier.Small, color = Cairn.colors.weak)
}

/**
 * A text field on the field rung (ADR-0037): the field colour inside the separator line, the same
 * 36 dp minimum height every control has. What is typed is drawn in the face that has each glyph,
 * and its direction follows its first strong character, as any text here does.
 */
@Composable
fun CairnTextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
) {
    val colors = Cairn.colors
    val faces = LocalFaces.current
    val shape = RoundedCornerShape(Controls.WIDGET_RADIUS.dp)
    val faced = remember(faces) {
        VisualTransformation { text ->
            TransformedText(faces.resolve(text.text, Family.Proportional), OffsetMapping.Identity)
        }
    }
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        singleLine = true,
        keyboardOptions = keyboardOptions,
        textStyle = TextStyle(
            color = colors.body,
            fontSize = Typography.BODY.sp,
            textDirection = TextDirection.Content,
        ),
        cursorBrush = SolidColor(colors.body),
        visualTransformation = faced,
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = Controls.HEIGHT.dp)
                    .clip(shape)
                    .background(colors.field)
                    .border(BorderStroke(1.dp, colors.separator), shape)
                    .padding(horizontal = Spacing.UNIT.dp),
                contentAlignment = Alignment.CenterStart,
                // The text takes the field's width, so a right-to-left line sits at the right edge.
                propagateMinConstraints = true,
            ) { inner() }
        },
    )
}
