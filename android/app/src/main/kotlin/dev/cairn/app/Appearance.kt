package dev.cairn.app

import android.graphics.drawable.ColorDrawable
import androidx.activity.compose.LocalActivity
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.foundation.isSystemInDarkTheme
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.ThemeChoice

/**
 * The appearance the app is drawn in (ADR-0036 §3): the stored choice, held as state so that the write
 * Settings makes changes the page, the window background and the system bar icons together.
 */
class AppearanceState(initial: ThemeChoice) {
    var choice: ThemeChoice by mutableStateOf(initial)
}

/** The state Settings writes to; the activity provides the real one. */
val LocalAppearance = staticCompositionLocalOf { AppearanceState(ThemeChoice.SYSTEM) }

/**
 * The theme of every screen, derived from [appearance] in one place: the page colours, the window
 * behind them and the system bars' icons all follow the same value, so they switch on the same frame.
 */
@Composable
fun CairnAppearance(appearance: AppearanceState, content: @Composable () -> Unit) {
    val dark = isDark(appearance.choice, isSystemInDarkTheme())
    val window = LocalActivity.current?.window
    SideEffect { window?.setBackgroundDrawable(ColorDrawable(pageArgb(dark))) }
    SystemBarIcons(dark)
    CompositionLocalProvider(LocalAppearance provides appearance) {
        CairnTheme(dark = dark, content = content)
    }
}
