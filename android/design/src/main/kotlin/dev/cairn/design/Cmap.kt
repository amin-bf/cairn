package dev.cairn.design

import java.nio.ByteBuffer

/**
 * Which code points one font file carries, read from its `cmap` table.
 *
 * It exists because the platform cannot answer this per face: `Paint.hasGlyph` asks the typeface
 * *with the system's fallback fonts appended*, so it says yes to a Persian letter on a face that has
 * none. Reading the file is the only way to know which of the bundled faces holds a glyph, and so
 * which one egui's first-match order would reach. Formats 4 and 12 cover every bundled face.
 */
class Cmap private constructor(private val ranges: List<LongRange>) {
    fun has(codePoint: Int): Boolean = ranges.any { codePoint.toLong() in it }

    companion object {
        fun of(font: ByteArray): Cmap {
            val buf = ByteBuffer.wrap(font)
            val tables = buf.getShort(4).toInt() and 0xFFFF
            var cmapOffset = -1
            for (i in 0 until tables) {
                val record = 12 + i * 16
                val tag = String(font, record, 4, Charsets.US_ASCII)
                if (tag == "cmap") cmapOffset = buf.getInt(record + 8)
            }
            require(cmapOffset >= 0) { "the font has no cmap table" }

            val ranges = ArrayList<LongRange>()
            val subtables = buf.getShort(cmapOffset + 2).toInt() and 0xFFFF
            for (i in 0 until subtables) {
                val record = cmapOffset + 4 + i * 8
                val platform = buf.getShort(record).toInt() and 0xFFFF
                val encoding = buf.getShort(record + 2).toInt() and 0xFFFF
                val unicode = platform == 0 || (platform == 3 && (encoding == 1 || encoding == 10))
                if (!unicode) continue
                val start = cmapOffset + buf.getInt(record + 4)
                when (buf.getShort(start).toInt() and 0xFFFF) {
                    4 -> format4(buf, start, ranges)
                    12 -> format12(buf, start, ranges)
                }
            }
            return Cmap(ranges)
        }

        private fun format4(buf: ByteBuffer, start: Int, out: MutableList<LongRange>) {
            val segments = (buf.getShort(start + 6).toInt() and 0xFFFF) / 2
            val ends = start + 14
            val starts = ends + segments * 2 + 2
            val deltas = starts + segments * 2
            val offsets = deltas + segments * 2
            for (s in 0 until segments) {
                val first = buf.getShort(starts + s * 2).toInt() and 0xFFFF
                val last = buf.getShort(ends + s * 2).toInt() and 0xFFFF
                if (first == 0xFFFF && last == 0xFFFF) continue
                val rangeOffset = buf.getShort(offsets + s * 2).toInt() and 0xFFFF
                val delta = buf.getShort(deltas + s * 2).toInt()
                // A range maps to glyph 0 (no glyph) when its delta lands there; keep the code points
                // whose glyph is not 0.
                var run = -1
                for (c in first..last) {
                    val glyph = if (rangeOffset == 0) {
                        (c + delta) and 0xFFFF
                    } else {
                        val at = offsets + s * 2 + rangeOffset + (c - first) * 2
                        val raw = buf.getShort(at).toInt() and 0xFFFF
                        if (raw == 0) 0 else (raw + delta) and 0xFFFF
                    }
                    if (glyph != 0) {
                        if (run < 0) run = c
                    } else if (run >= 0) {
                        out += run.toLong()..(c - 1).toLong()
                        run = -1
                    }
                }
                if (run >= 0) out += run.toLong()..last.toLong()
            }
        }

        private fun format12(buf: ByteBuffer, start: Int, out: MutableList<LongRange>) {
            val groups = buf.getInt(start + 12)
            for (g in 0 until groups) {
                val at = start + 16 + g * 12
                val first = buf.getInt(at).toLong() and 0xFFFFFFFFL
                val last = buf.getInt(at + 4).toLong() and 0xFFFFFFFFL
                val glyph = buf.getInt(at + 8)
                if (glyph != 0) out += first..last
            }
        }
    }
}
