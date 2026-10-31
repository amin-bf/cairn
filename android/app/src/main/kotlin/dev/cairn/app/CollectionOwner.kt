package dev.cairn.app

import android.content.Context
import android.os.Looper
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.ThemeChoice
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File

/** What a screen's model needs of the collection: a way to run a block on it, off the main thread. */
interface CollectionAccess {
    suspend fun <T> use(block: (CollectionHandle) -> T): T
}

/** The stored appearance; a failed read follows the platform rather than leaving a window blank. */
suspend fun CollectionAccess.themeChoiceOrSystem(): ThemeChoice =
    try {
        use { it.themeChoice() }
    } catch (_: CairnException) {
        ThemeChoice.SYSTEM
    }

/**
 * The one collection of the process (ADR-0046 §7): opened once, held for the process's life, and
 * reached only from a coroutine on [Dispatchers.IO] — a synchronous call into the binding blocks the
 * thread it is made on, and StrictMode in debug builds fails any that reach the main thread.
 *
 * Kotlin passes `filesDir` and `noBackupFilesDir` to the open operation (ADR-0046 §5); the Rust side
 * never asks the platform for them. Everything Cairn persists goes through the store (§6), so this
 * owner keeps no file of its own.
 */
class CollectionOwner(
    private val context: Context,
    // Where the collection lives: the app's two directories unless a test gives its own. They are
    // resolved when the collection is first opened, because asking the platform for one is disk work.
    private val dataDir: File? = null,
    private val stateDir: File? = null,
) : CollectionAccess {
    private val lock = Mutex()
    private var handle: CollectionHandle? = null

    /** Run [block] on the collection, off the main thread. The first call opens it. */
    override suspend fun <T> use(block: (CollectionHandle) -> T): T = withContext(Dispatchers.IO) {
        block(opened())
    }

    private suspend fun opened(): CollectionHandle = lock.withLock {
        check(Looper.myLooper() != Looper.getMainLooper()) { "the collection was reached on the main thread" }
        handle ?: CollectionHandle.open(
            (dataDir ?: context.filesDir).absolutePath,
            (stateDir ?: context.noBackupFilesDir).absolutePath,
        ).also { handle = it }
    }
}
