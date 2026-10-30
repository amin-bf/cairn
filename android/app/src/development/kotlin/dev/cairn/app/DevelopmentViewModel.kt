package dev.cairn.app

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.BenchFixture
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.fixtures
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/**
 * What the bench last said, kept outside every screen: an install starts the app afresh, which drops
 * the Settings page that asked, and the answer is for the person who comes back to it. It stays until
 * the next press (#237).
 */
object DevelopmentBench {
    var said: String? by mutableStateOf(null)
}

/** The row that is asking before it replaces the collection. */
sealed interface Asking {
    data object Reset : Asking

    data class Install(val key: String) : Asking
}

/**
 * The Development block's state (ADR-0051 §1, §6): the bench's list, and the one row that is asking.
 * Reset and an install are the binding's; this asks first, then starts the app afresh when the
 * collection was replaced.
 */
class DevelopmentViewModel(
    private val owner: CollectionAccess,
    private val wallClock: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    val fixtures: List<BenchFixture> = fixtures()

    private val _asking = MutableStateFlow<Asking?>(null)
    val asking: StateFlow<Asking?> = _asking

    /** A press on a row's control turns that row into its question; any other row's question is dropped. */
    fun ask(row: Asking) {
        _asking.value = row
        DevelopmentBench.said = null
    }

    /** *Keep it*: nothing changes. */
    fun keep() {
        _asking.value = null
    }

    /** *Reset*: the collection returns to a first launch, and then the app starts afresh. */
    fun reset(onRestart: () -> Unit) {
        _asking.value = null
        viewModelScope.launch {
            try {
                owner.use { it.reset() }
                onRestart()
            } catch (e: CairnException) {
                DevelopmentBench.said = refusal(e)
            }
        }
    }

    /** *Replace*: the collection becomes the fixture, the bench says what it reached, and the app starts afresh. */
    fun install(key: String, onRestart: () -> Unit) {
        _asking.value = null
        viewModelScope.launch {
            try {
                val reached = owner.use { it.installFixture(key, wallClock()) }
                DevelopmentBench.said = "$key — $reached"
                onRestart()
            } catch (e: CairnException) {
                DevelopmentBench.said = refusal(e)
            }
        }
    }

    private fun refusal(e: CairnException): String = e.message ?: e.toString()
}
