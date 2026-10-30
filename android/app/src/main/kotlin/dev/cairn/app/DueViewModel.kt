package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

sealed interface DueState {
    data object Loading : DueState

    data class Known(val due: Int) : DueState

    data object Failed : DueState
}

/** Reads the due count through the binding, off the main thread (the owner moves every call to IO). */
class DueViewModel(private val owner: CollectionOwner) : ViewModel() {
    private val _state = MutableStateFlow<DueState>(DueState.Loading)
    val state: StateFlow<DueState> = _state

    init {
        viewModelScope.launch {
            _state.value = try {
                DueState.Known(owner.use { it.dueCount(System.currentTimeMillis()) }.toInt())
            } catch (_: CairnException) {
                DueState.Failed
            }
        }
    }
}
