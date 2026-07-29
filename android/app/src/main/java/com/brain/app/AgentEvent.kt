package com.brain.app

import kotlinx.serialization.*
import kotlinx.serialization.json.*

@Serializable
sealed class AgentEvent {
    abstract val type: String
    abstract val ts: Long
}

@Serializable
data class ThoughtEvent(
    override val type: String = "thought",
    override val ts: Long = 0,
    val text: String = ""
) : AgentEvent()

@Serializable
data class ToolCallEvent(
    override val type: String = "tool_call",
    override val ts: Long = 0,
    val tool: String = "",
    val args: JsonObject = JsonObject(emptyMap()),
    val call_id: String = ""
) : AgentEvent()

@Serializable
data class ToolResultEvent(
    override val type: String = "tool_result",
    override val ts: Long = 0,
    val call_id: String = "",
    val success: Boolean = false,
    val summary: String = ""
) : AgentEvent()

@Serializable
data class TodoUpdateEvent(
    override val type: String = "todo_update",
    override val ts: Long = 0,
    val todos: List<TodoItem> = emptyList()
) : AgentEvent()

@Serializable
data class TodoItem(
    val id: String = "",
    val text: String = "",
    val status: String = ""
)

@Serializable
data class FileReadEvent(
    override val type: String = "file_read",
    override val ts: Long = 0,
    val path: String = "",
    val text: String = ""
) : AgentEvent()

@Serializable
data class DoneEvent(
    override val type: String = "done",
    override val ts: Long = 0,
    val summary: String = "",
    val total_tokens: Int = 0,
    val total_calls: Int = 0
) : AgentEvent()

@Serializable
data class ErrorEvent(
    override val type: String = "error",
    override val ts: Long = 0,
    val message: String = ""
) : AgentEvent()

@Serializable
data class TaskRequest(
    val task: String,
    val mode: String = "auto"
)

fun parseAgentEvent(json: String): AgentEvent {
    val obj = Json.parseToJsonElement(json).jsonObject
    return when (obj["type"]?.jsonPrimitive?.content) {
        "thought" -> Json.decodeFromJsonElement<ThoughtEvent>(obj)
        "tool_call" -> Json.decodeFromJsonElement<ToolCallEvent>(obj)
        "tool_result" -> Json.decodeFromJsonElement<ToolResultEvent>(obj)
        "todo_update" -> Json.decodeFromJsonElement<TodoUpdateEvent>(obj)
        "file_read" -> Json.decodeFromJsonElement<FileReadEvent>(obj)
        "done" -> Json.decodeFromJsonElement<DoneEvent>(obj)
        "error" -> Json.decodeFromJsonElement<ErrorEvent>(obj)
        else -> ThoughtEvent(text = "[unknown event: ${obj["type"]}]", ts = System.currentTimeMillis() / 1000)
    }
}
