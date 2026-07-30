package com.brain.app.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.brain.app.data.*
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class BrainViewModel(
    application: Application,
    private val repository: AgentRepository,
    val settings: BrainSettings
) : AndroidViewModel(application) {

    private val _events = MutableStateFlow<List<AgentEvent>>(emptyList())
    val events: StateFlow<List<AgentEvent>> = _events.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private val _availableModels = MutableStateFlow<List<String>>(emptyList())
    val availableModels: StateFlow<List<String>> = _availableModels.asStateFlow()

    private val _selectedModel = MutableStateFlow("")
    val selectedModel: StateFlow<String> = _selectedModel.asStateFlow()

    private var wsJob: Job? = null

    init {
        _selectedModel.value = settings.llmModel.value
    }

    fun selectModel(model: String) {
        _selectedModel.value = model
        settings.saveModels(model, "")
    }

    fun sendTask(task: String) {
        if (task.isBlank()) return

        // Add user event for display
        _events.value = _events.value + AgentEvent.Thought(text = task)

        wsJob?.cancel()
        _isRunning.value = true

        wsJob = viewModelScope.launch {
            try {
                repository.connect(task).collect { event ->
                    _events.value = _events.value + event
                    if (event is AgentEvent.Done || event is AgentEvent.Error) {
                        _isRunning.value = false
                    }
                }
            } catch (e: Exception) {
                _events.value = _events.value + AgentEvent.Error(
                    message = e.message ?: "Connection error"
                )
                _isRunning.value = false
            }
        }
    }

    fun stopAgent() {
        wsJob?.cancel()
        wsJob = null
        _isRunning.value = false
    }

    fun newChat() {
        _events.value = emptyList()
        _isRunning.value = false
        wsJob?.cancel()
        wsJob = null
    }

    override fun onCleared() {
        wsJob?.cancel()
    }
}
