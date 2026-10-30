package dev.cairn.design

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.runtime.remember
import dev.cairn.design.tokens.DarkPalette
import dev.cairn.design.tokens.LightPalette

/** Every colour role, for one theme. Screens read these and never write a colour (rule 13). */
@Stable
class CairnColors internal constructor(
    val dark: Boolean,
    val page: Color,
    val popup: Color,
    val popupStroke: Color,
    val field: Color,
    val control: Color,
    val card: Color,
    val cardDivider: Color,
    val separator: Color,
    val body: Color,
    val weak: Color,
    val onControl: Color,
    val primary: Color,
    val primaryStroke: Color,
    val hover: Color,
    val hoverStroke: Color,
    val pressed: Color,
    val selection: Color,
    val selectionStroke: Color,
    val link: Color,
    val warn: Color,
    val error: Color,
)

internal val darkColors = CairnColors(
    dark = true,
    page = Color(DarkPalette.PAGE),
    popup = Color(DarkPalette.POPUP),
    popupStroke = Color(DarkPalette.POPUP_STROKE),
    field = Color(DarkPalette.FIELD),
    control = Color(DarkPalette.CONTROL),
    card = Color(DarkPalette.CARD),
    cardDivider = Color(DarkPalette.CARD_DIVIDER),
    separator = Color(DarkPalette.SEPARATOR),
    body = Color(DarkPalette.BODY),
    weak = Color(DarkPalette.WEAK),
    onControl = Color(DarkPalette.ON_CONTROL),
    primary = Color(DarkPalette.PRIMARY),
    primaryStroke = Color(DarkPalette.PRIMARY_STROKE),
    hover = Color(DarkPalette.HOVER),
    hoverStroke = Color(DarkPalette.HOVER_STROKE),
    pressed = Color(DarkPalette.PRESSED),
    selection = Color(DarkPalette.SELECTION),
    selectionStroke = Color(DarkPalette.SELECTION_STROKE),
    link = Color(DarkPalette.LINK),
    warn = Color(DarkPalette.WARN),
    error = Color(DarkPalette.ERROR),
)

internal val lightColors = CairnColors(
    dark = false,
    page = Color(LightPalette.PAGE),
    popup = Color(LightPalette.POPUP),
    popupStroke = Color(LightPalette.POPUP_STROKE),
    field = Color(LightPalette.FIELD),
    control = Color(LightPalette.CONTROL),
    card = Color(LightPalette.CARD),
    cardDivider = Color(LightPalette.CARD_DIVIDER),
    separator = Color(LightPalette.SEPARATOR),
    body = Color(LightPalette.BODY),
    weak = Color(LightPalette.WEAK),
    onControl = Color(LightPalette.ON_CONTROL),
    primary = Color(LightPalette.PRIMARY),
    primaryStroke = Color(LightPalette.PRIMARY_STROKE),
    hover = Color(LightPalette.HOVER),
    hoverStroke = Color(LightPalette.HOVER_STROKE),
    pressed = Color(LightPalette.PRESSED),
    selection = Color(LightPalette.SELECTION),
    selectionStroke = Color(LightPalette.SELECTION_STROKE),
    link = Color(LightPalette.LINK),
    warn = Color(LightPalette.WARN),
    error = Color(LightPalette.ERROR),
)

/** The page colour as an ARGB int, for the window behind the first frame (a view, not a composable). */
fun pageArgb(dark: Boolean): Int = (if (dark) DarkPalette.PAGE else LightPalette.PAGE).toInt()

val LocalCairnColors =staticCompositionLocalOf { darkColors }
internal val LocalFaces = staticCompositionLocalOf<FaceSet> { error("CairnTheme is missing") }

/** The theme as the roles of the moment, read from [CairnTheme]. */
object Cairn {
    val colors: CairnColors
        @Composable @ReadOnlyComposable get() = LocalCairnColors.current
}

/**
 * Carries the colours and the faces. [dark] is decided by the caller from the stored choice and the
 * platform (`Light` and `Dark` are fixed, `System` follows the platform), so this never interprets
 * the stored value (ADR-0046 §1, §6).
 */
@Composable
fun CairnTheme(dark: Boolean, content: @Composable () -> Unit) {
    val context = LocalContext.current
    val faces = remember(context) { FaceSet(context) }
    CompositionLocalProvider(
        LocalCairnColors provides (if (dark) darkColors else lightColors),
        LocalFaces provides faces,
        content = content,
    )
}
