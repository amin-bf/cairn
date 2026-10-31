package dev.cairn.app

import android.content.ClipData
import android.content.Intent
import android.net.Uri

/**
 * The system share sheet over a saved document (ADR-0023 §3 §7, ADR-0047 §4): a chooser around an
 * `ACTION_SEND` carrying [document] as its stream, typed `application/octet-stream` as the document was
 * created (deck export rule 12). `createChooser` returns a fresh intent that inherits none of the inner
 * one's flags, so the read grant and the URI as `ClipData` go on the chooser itself; without them the
 * grant fails only after a target was picked. No target is counted or named, so no `<queries>` is needed.
 */
fun shareSheet(document: Uri): Intent {
    val send = Intent(Intent.ACTION_SEND)
        .setType("application/octet-stream")
        .putExtra(Intent.EXTRA_STREAM, document)
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    return Intent.createChooser(send, null).apply {
        clipData = ClipData.newRawUri(null, document)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }
}
