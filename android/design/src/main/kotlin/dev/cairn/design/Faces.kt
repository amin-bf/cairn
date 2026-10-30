package dev.cairn.design

import android.content.Context
import android.graphics.Typeface
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import dev.cairn.design.tokens.Faces

/** The three families the desktop draws with (`fonts::families`). */
enum class Family { Proportional, Monospace, Bold }

/**
 * The bundled faces, and the fallback Compose does not have.
 *
 * A Compose `FontFamily` picks one face for a whole span; egui reaches the next face for each
 * missing glyph. So [resolve] walks the text one code point at a time, in egui's order for the
 * family, and puts each run in the first face that has its glyph (read from the font file, see
 * [Cmap]; the platform's own `Paint.hasGlyph` counts the system fallback fonts). The order is
 * the one in [Faces], which the guard test holds equal to the desktop's: it decides which face is
 * reached, and a Persian letter must reach Noto Sans Arabic before DejaVu's afterthought of one
 * (client-stack rule 7).
 */
class FaceSet(context: Context) {
    private val assets = context.applicationContext.assets
    private val loaded = HashMap<String, Typeface>()
    private val chain = HashMap<Family, List<Typeface>>()
    private val covers = HashMap<String, Cmap>()

    private fun files(family: Family): List<String> =
        when (family) {
            Family.Proportional -> Faces.PROPORTIONAL
            Family.Monospace -> Faces.MONOSPACE
            Family.Bold -> Faces.BOLD
        }.split(',')

    private fun typeface(file: String): Typeface =
        loaded.getOrPut(file) { Typeface.createFromAsset(assets, "fonts/$file") }

    private fun cmap(file: String): Cmap =
        covers.getOrPut(file) { Cmap.of(assets.open("fonts/$file").use { it.readBytes() }) }

    /** The family's faces in the order it reaches them. */
    fun faces(family: Family): List<Typeface> =
        chain.getOrPut(family) { files(family).map(::typeface) }

    /**
     * For each code point of [text], the place in [faces] of the face that draws it. A combining mark
     * rides the face of the letter it sits on, and a code point no face carries (a zero-width joiner,
     * say) stays in the run it is inside; a text that starts with one gets the first face.
     */
    fun owners(text: String, family: Family): List<Int> {
        val maps = files(family).map(::cmap)
        val owners = ArrayList<Int>()
        var current = 0
        var i = 0
        while (i < text.length) {
            val cp = text.codePointAt(i)
            val width = Character.charCount(cp)
            val kind = Character.getType(cp)
            val joins = kind == Character.NON_SPACING_MARK.toInt() || kind == Character.ENCLOSING_MARK.toInt()
            if (!(joins && owners.isNotEmpty())) {
                val found = maps.indexOfFirst { it.has(cp) }
                if (found >= 0) current = found
            }
            owners += current
            i += width
        }
        return owners
    }

    /** [text] as one run per face, which is how Compose is told to draw each glyph in the face that has it. */
    fun resolve(text: String, family: Family): AnnotatedString {
        val faces = faces(family)
        val owners = owners(text, family)
        return buildAnnotatedString {
            var i = 0
            var cp = 0
            while (i < text.length) {
                val owner = owners[cp]
                val start = i
                while (i < text.length && owners[cp] == owner) {
                    i += Character.charCount(text.codePointAt(i))
                    cp += 1
                }
                pushStyle(SpanStyle(fontFamily = FontFamily(faces[owner])))
                append(text.substring(start, i))
                pop()
            }
        }
    }
}
