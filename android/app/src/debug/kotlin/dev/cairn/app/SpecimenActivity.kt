package dev.cairn.app

import androidx.compose.runtime.Composable
import dev.cairn.design.Specimen

/** Debug builds only: the design module's specimen, to look at the faces and the theme on a handset. */
class SpecimenActivity : CairnActivity() {
    @Composable
    override fun Screen() = Specimen()
}
