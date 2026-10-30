package dev.cairn.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import dev.cairn.ffi.ThemeChoice

/**
 * The app's navigation over a collection a test hands it, with its content set in onCreate as the real
 * activity's is, so that a recreation restores the back stack and keeps the models (SettingsRotationTest).
 */
class RotatingActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        setContent { CairnAppearance(appearance) { CairnNavHost(access!!, keyboardUp = false) } }
    }

    companion object {
        @Volatile
        var access: CollectionAccess? = null
    }
}
