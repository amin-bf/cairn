package dev.cairn.app

import android.content.res.Configuration
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import android.view.ViewTreeObserver
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import androidx.lifecycle.lifecycleScope
import dev.cairn.design.pageArgb
import dev.cairn.ffi.ThemeChoice
import kotlinx.coroutines.launch

/** Whether the screen is dark: `Light` and `Dark` are fixed, `System` follows the platform. */
fun isDark(choice: ThemeChoice, systemDark: Boolean): Boolean =
    when (choice) {
        ThemeChoice.SYSTEM -> systemDark
        ThemeChoice.LIGHT -> false
        ThemeChoice.DARK -> true
    }

/**
 * The base of every screen of the app. It reads the stored appearance choice through the binding
 * before anything is drawn: the window's first draw is held back until the choice is known, so a
 * handset set to dark never shows a light frame first (ADR-0036 §3). The choice is only read here;
 * Settings changes it through [AppearanceState].
 */
abstract class CairnActivity : ComponentActivity() {
    private var ready = false

    @Composable
    protected abstract fun Screen()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val content = window.decorView
        content.viewTreeObserver.addOnPreDrawListener(
            object : ViewTreeObserver.OnPreDrawListener {
                override fun onPreDraw(): Boolean {
                    if (!ready) return false
                    content.viewTreeObserver.removeOnPreDrawListener(this)
                    return true
                }
            },
        )
        lifecycleScope.launch {
            val choice = (application as CairnApplication).collection.themeChoiceOrSystem()
            // The window theme follows the platform, so a stored choice that differs from it is
            // put behind the first frame here.
            val systemDark = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
                Configuration.UI_MODE_NIGHT_YES
            window.setBackgroundDrawable(ColorDrawable(pageArgb(isDark(choice, systemDark))))
            val appearance = AppearanceState(choice)
            setContent { CairnAppearance(appearance) { Screen() } }
            ready = true
        }
    }
}

/**
 * The system bars' icons follow the theme, as the platform's practice is (ADR-0045 §7): dark icons
 * on the light page, light icons on the dark one. The window theme asks for neither, and the
 * activity draws edge to edge, so the page shows behind the bars. It runs on every change of
 * [dark], so a switch of the phone's theme while the app is open changes them too.
 */
@Composable
internal fun SystemBarIcons(dark: Boolean) {
    val view = LocalView.current
    val window = LocalActivity.current?.window ?: return
    SideEffect {
        val bars = WindowCompat.getInsetsController(window, view)
        bars.isAppearanceLightStatusBars = !dark
        bars.isAppearanceLightNavigationBars = !dark
    }
}
