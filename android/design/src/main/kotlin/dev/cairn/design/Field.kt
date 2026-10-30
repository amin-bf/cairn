package dev.cairn.design

import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.sp
import dev.cairn.design.tokens.Typography
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.platform.LocalLayoutDirection
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
    keyboardActions: KeyboardActions = KeyboardActions.Default,
    singleLine: Boolean = true,
) {
    val colors = Cairn.colors
    val faces = LocalFaces.current
    val faced = remember(faces) {
        VisualTransformation { text ->
            TransformedText(faces.resolve(text.text, Family.Proportional), OffsetMapping.Identity)
        }
    }
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        singleLine = singleLine,
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
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
                    .controlSurface(colors.field, colors.separator)
                    .padding(horizontal = gap(1)),
                contentAlignment = Alignment.CenterStart,
                // The text takes the field's width, so a right-to-left line sits at the right edge.
                propagateMinConstraints = true,
            ) { inner() }
        },
    )
}

/**
 * The same field over a [TextFieldValue], for a caller that needs the selection (*Blank it*) or keeps
 * the composition itself. The look is [CairnTextField]'s.
 */
@Composable
fun CairnTextField(
    value: TextFieldValue,
    onValueChange: (TextFieldValue) -> Unit,
    modifier: Modifier = Modifier,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
    singleLine: Boolean = true,
) {
    val colors = Cairn.colors
    val faces = LocalFaces.current
    val faced = remember(faces) {
        VisualTransformation { text ->
            TransformedText(faces.resolve(text.text, Family.Proportional), OffsetMapping.Identity)
        }
    }
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        singleLine = singleLine,
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
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
                    .controlSurface(colors.field, colors.separator)
                    .padding(horizontal = gap(1)),
                contentAlignment = Alignment.CenterStart,
                propagateMinConstraints = true,
            ) { inner() }
        },
    )
}

/**
 * Whether [text] sets right to left: its first strong character decides, as the platform's own
 * content-direction rule does. Empty text, or text with no strong character, is left to right.
 */
fun isRightToLeft(text: String): Boolean {
    var at = 0
    while (at < text.length) {
        val point = text.codePointAt(at)
        when (Character.getDirectionality(point)) {
            Character.DIRECTIONALITY_RIGHT_TO_LEFT, Character.DIRECTIONALITY_RIGHT_TO_LEFT_ARABIC -> return true
            Character.DIRECTIONALITY_LEFT_TO_RIGHT -> return false
        }
        at += Character.charCount(point)
    }
    return false
}

/**
 * A field with its [label] over it (ADR-0040 §6). The pair is laid out by the first strong character
 * of what the field holds, so a Persian field's label sits over the edge its text starts at — the
 * right — and a French one's over the left.
 */
@Composable
fun CairnLabeledField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
    singleLine: Boolean = true,
) {
    val direction = if (isRightToLeft(value)) LayoutDirection.Rtl else LayoutDirection.Ltr
    CompositionLocalProvider(LocalLayoutDirection provides direction) {
        Column(Modifier.fillMaxWidth()) {
            CairnFieldLabel(label, Modifier.testTag("label-$label"))
            CairnTextField(value, onValueChange, modifier, keyboardOptions, keyboardActions, singleLine)
        }
    }
}

/** [CairnLabeledField] over a [TextFieldValue]: the label and the field laid out by the first strong character. */
@Composable
fun CairnLabeledField(
    label: String,
    value: TextFieldValue,
    onValueChange: (TextFieldValue) -> Unit,
    modifier: Modifier = Modifier,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
    singleLine: Boolean = true,
) {
    val direction = if (isRightToLeft(value.text)) LayoutDirection.Rtl else LayoutDirection.Ltr
    CompositionLocalProvider(LocalLayoutDirection provides direction) {
        Column(Modifier.fillMaxWidth()) {
            CairnFieldLabel(label, Modifier.testTag("label-$label"))
            CairnTextField(value, onValueChange, modifier, keyboardOptions, keyboardActions, singleLine)
        }
    }
}

/**
 * [text] with the stretch between [start] and [end] (in either order) wrapped as the blank [number],
 * `{{n::…}}` (ADR-0012 §3), and where the caret goes: just after the closing braces. Which number is
 * the next one is Rust's to say; this only does the wrapping.
 */
fun wrapAsBlank(text: String, start: Int, end: Int, number: Int): Pair<String, Int> {
    val from = minOf(start, end)
    val to = maxOf(start, end)
    val open = "{{$number::"
    val wrapped = text.substring(0, from) + open + text.substring(from, to) + "}}" + text.substring(to)
    return wrapped to (to + open.length + 2)
}
