package com.brain.app

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class AgentViewModel(
    application: Application,
    private val repository: AgentRepository,
    private val settings: BrainSettings
) : AndroidViewModel(application) {

    private val chatRepo = ChatRepository(application)

    private val _chats = MutableStateFlow<List<ChatSession>>(emptyList())
    val chats: StateFlow<List<ChatSession>> = _chats.asStateFlow()

    private val _currentChatId = MutableStateFlow<String?>(null)
    val currentChatId: StateFlow<String?> = _currentChatId.asStateFlow()

    private val _events = MutableStateFlow<List<AgentEvent>>(emptyList())
    val events: StateFlow<List<AgentEvent>> = _events.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private var wsJob: Job? = null

    init {
        _chats.value = chatRepo.loadChats()
        if (_chats.value.isNotEmpty()) {
            _currentChatId.value = _chats.value.first().id
            _events.value = _chats.value.first().messages.flatMap { msg ->
                msg.events?.map { deserializedToEvent(it) } ?: listOf(
                    ThoughtEvent(text = msg.content, ts = msg.timestamp)
                )
            }
        }
    }

    fun newChat() {
        val chat = ChatSession(
            title = "New chat",
            model = settings.llmModel.value
        )
        _chats.value = listOf(chat) + _chats.value
        _currentChatId.value = chat.id
        _events.value = emptyList()
        save()
    }

    fun selectChat(id: String) {
        val chat = _chats.value.find { it.id == id } ?: return
        _currentChatId.value = id
        _events.value = chat.messages.flatMap { msg ->
            if (msg.events != null) {
                msg.events.map { deserializedToEvent(it) }
            } else {
                listOf(ThoughtEvent(text = msg.content, ts = msg.timestamp))
            }
        }
    }

    fun deleteChat(id: String) {
        _chats.value = _chats.value.filter { it.id != id }
        if (_currentChatId.value == id) {
            if (_chats.value.isNotEmpty()) {
                _currentChatId.value = _chats.value.first().id
                selectChat(_currentChatId.value!!)
            } else {
                newChat()
            }
        }
        save()
    }

    fun sendTask(task: String) {
        if (task.isBlank()) return

        val currentId = _currentChatId.value ?: run {
            newChat()
            _currentChatId.value!!
        }

        // Add user message event for immediate display
        val userMsg = ChatMessage(
            role = "user",
            content = task,
            events = listOf(SerializedEvent(type = "user_message", ts = System.currentTimeMillis() / 1000, text = task))
        )
        _events.value = _events.value + ThoughtEvent(text = task, ts = System.currentTimeMillis() / 1000)

        wsJob?.cancel()
        _isRunning.value = true

        wsJob = viewModelScope.launch {
            val agentEvents = mutableListOf<AgentEvent>()
            try {
                repository.connect(task).collect { event ->
                    _events.value = _events.value + event
                    agentEvents.add(event)
                    if (event is DoneEvent || event is ErrorEvent) {
                        _isRunning.value = false
                    }
                }
            } catch (e: Exception) {
                _events.value = _events.value + ErrorEvent(
                    message = e.message ?: "Connection error",
                    ts = System.currentTimeMillis() / 1000
                )
                _isRunning.value = false
            }

            // Persist chat
            val current = _chats.value.find { it.id == currentId }
            if (current != null) {
                val assistantMsg = ChatMessage(
                    role = "assistant",
                    content = agentEvents.filterIsInstance<DoneEvent>().firstOrNull()?.summary ?: "",
                    events = agentEvents.map { it.toSerialized() }
                )
                val newMessages = current.messages + userMsg + assistantMsg
                val title = if (current.messages.isEmpty()) {
                    task.take(50)
                } else current.title
                val updated = current.copy(
                    messages = newMessages,
                    title = title,
                    updatedAt = System.currentTimeMillis()
                )
                _chats.value = _chats.value.map { if (it.id == currentId) updated else it }
                save()
            }
        }
    }

    fun stopAgent() {
        wsJob?.cancel()
        wsJob = null
        _isRunning.value = false
    }

    private fun save() {
        chatRepo.saveChats(_chats.value)
    }

    private fun deserializedToEvent(se: SerializedEvent): AgentEvent = when (se.type) {
        "thought" -> ThoughtEvent(text = se.text, ts = se.ts)
        "tool_call" -> ToolCallEvent(tool = se.tool, call_id = se.call_id, ts = se.ts)
        "tool_result" -> ToolResultEvent(call_id = se.call_id, success = se.success, summary = se.summary, ts = se.ts)
        "file_read" -> FileReadEvent(path = se.path, text = se.text, ts = se.ts)
        "done" -> DoneEvent(summary = se.summary, total_tokens = se.total_tokens, total_calls = se.total_calls, ts = se.ts)
        "error" -> ErrorEvent(message = se.message, ts = se.ts)
        "user_message" -> ThoughtEvent(text = se.text, ts = se.ts)
        else -> ThoughtEvent(text = se.text, ts = se.ts)
    }

    override fun onCleared() {
        wsJob?.cancel()
    }
}
