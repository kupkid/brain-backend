package com.brain.app.data

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

sealed class AgentEvent {
    data class Thought(val text: String, val ts: Long = now()) : AgentEvent()
    data class Text(val text: String, val ts: Long = now()) : AgentEvent()
    data class ToolCall(val tool: String, val callId: String, val args: String = "", val ts: Long = now()) : AgentEvent()
    data class ToolResult(val callId: String, val success: Boolean, val summary: String = "", val ts: Long = now()) : AgentEvent()
    data class FileRead(val path: String, val text: String, val ts: Long = now()) : AgentEvent()
    data class TodoUpdate(val tasks: List<TodoTask> = emptyList(), val ts: Long = now()) : AgentEvent()
    data class Done(
        val summary: String = "",
        val totalTokens: Int = 0,
        val totalCalls: Int = 0,
        val tokensInput: Int = 0,
        val tokensOutput: Int = 0,
        val elapsedMs: Long = 0,
        val tokensPerSec: Float = 0f,
        val ts: Long = now()
    ) : AgentEvent()
    data class Error(val message: String, val ts: Long = now()) : AgentEvent()
    data class Init(val status: String = "", val ts: Long = now()) : AgentEvent()

    companion object {
        fun now() = System.currentTimeMillis() / 1000
    }
}

data class TodoTask(
    val id: String = "",
    val title: String = "",
    val status: String = "pending"
)

data class TaskRequest(
    val task: String,
    val mode: String? = null
)

fun parseAgentEvent(json: String): AgentEvent {
    return try {
        val obj = Json.parseToJsonElement(json).jsonObject
        val type = obj["type"]?.jsonPrimitive?.content ?: ""
        val ts = obj["ts"]?.jsonPrimitive?.content?.toLongOrNull() ?: AgentEvent.now()

        when (type) {
            "thought" -> AgentEvent.Thought(text = obj["text"]?.jsonPrimitive?.content ?: "", ts = ts)
            "text" -> AgentEvent.Text(text = obj["text"]?.jsonPrimitive?.content ?: "", ts = ts)
            "tool_call" -> AgentEvent.ToolCall(
                tool = obj["tool"]?.jsonPrimitive?.content ?: "",
                callId = obj["call_id"]?.jsonPrimitive?.content ?: "",
                args = obj["args"]?.jsonPrimitive?.content ?: "",
                ts = ts
            )
            "tool_result" -> AgentEvent.ToolResult(
                callId = obj["call_id"]?.jsonPrimitive?.content ?: "",
                success = obj["success"]?.jsonPrimitive?.content?.toBooleanStrictOrNull() ?: true,
                summary = obj["summary"]?.jsonPrimitive?.content ?: "",
                ts = ts
            )
            "file_read" -> AgentEvent.FileRead(
                path = obj["path"]?.jsonPrimitive?.content ?: "",
                text = obj["text"]?.jsonPrimitive?.content ?: "",
                ts = ts
            )
            "todo_update" -> AgentEvent.TodoUpdate(ts = ts)
            "done" -> AgentEvent.Done(
                summary = obj["summary"]?.jsonPrimitive?.content ?: "",
                totalTokens = obj["total_tokens"]?.jsonPrimitive?.content?.toIntOrNull() ?: 0,
                totalCalls = obj["total_calls"]?.jsonPrimitive?.content?.toIntOrNull() ?: 0,
                tokensInput = obj["tokens_input"]?.jsonPrimitive?.content?.toIntOrNull() ?: 0,
                tokensOutput = obj["tokens_output"]?.jsonPrimitive?.content?.toIntOrNull() ?: 0,
                elapsedMs = obj["elapsed_ms"]?.jsonPrimitive?.content?.toLongOrNull() ?: 0,
                tokensPerSec = obj["tokens_per_sec"]?.jsonPrimitive?.content?.toFloatOrNull() ?: 0f,
                ts = ts
            )
            "error" -> AgentEvent.Error(message = obj["message"]?.jsonPrimitive?.content ?: "Unknown error", ts = ts)
            "init" -> AgentEvent.Init(status = obj["status"]?.jsonPrimitive?.content ?: "", ts = ts)
            "warning" -> AgentEvent.Error(message = obj["message"]?.jsonPrimitive?.content ?: "Warning", ts = ts)
            else -> AgentEvent.Thought(text = "[unknown: $json]")
        }
    } catch (e: Exception) {
        AgentEvent.Thought(text = json)
    }
}
