package com.brain.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class AgentViewModel(private val repository: AgentRepository) : ViewModel() {

    private val _events = MutableStateFlow<List<AgentEvent>>(emptyList())
    val events: StateFlow<List<AgentEvent>> = _events.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private var wsJob: Job? = null

    fun sendTask(task: String) {
        if (task.isBlank()) return

        _events.value = emptyList()
        _isRunning.value = true

        wsJob?.cancel()
        wsJob = viewModelScope.launch {
            repository.connect(task).collect { event ->
                _events.value = _events.value + event
                if (event is DoneEvent || event is ErrorEvent) {
                    _isRunning.value = false
                }
            }
        }
    }

    fun stopAgent() {
        wsJob?.cancel()
        wsJob = null
        _isRunning.value = false
        _events.value = _events.value + ErrorEvent(
            message = "Остановлен пользователем",
            ts = System.currentTimeMillis() / 1000
        )
    }

    override fun onCleared() {
        wsJob?.cancel()
    }
}
