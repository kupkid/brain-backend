package com.brain.app

import android.content.Context
import android.content.SharedPreferences
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.util.UUID

@Serializable
data class ChatMessage(
    val id: String = UUID.randomUUID().toString(),
    val role: String,
    val content: String,
    val timestamp: Long = System.currentTimeMillis(),
    val events: List<SerializedEvent>? = null,
)

@Serializable
data class SerializedEvent(
    val type: String,
    val ts: Long,
    val text: String = "",
    val tool: String = "",
    val call_id: String = "",
    val success: Boolean = false,
    val summary: String = "",
    val path: String = "",
    val total_tokens: Int = 0,
    val total_calls: Int = 0,
    val message: String = "",
)

@Serializable
data class ChatSession(
    val id: String = UUID.randomUUID().toString(),
    val title: String = "New chat",
    val model: String = "",
    val messages: List<ChatMessage> = emptyList(),
    val createdAt: Long = System.currentTimeMillis(),
    val updatedAt: Long = System.currentTimeMillis(),
)

class ChatRepository(context: Context) {
    private val prefs: SharedPreferences = context.getSharedPreferences("brain_chats", Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    fun saveChats(chats: List<ChatSession>) {
        prefs.edit().putString("chats", json.encodeToString(chats)).apply()
    }

    fun loadChats(): List<ChatSession> {
        return try {
            val raw = prefs.getString("chats", null) ?: return emptyList()
            json.decodeFromString<List<ChatSession>>(raw)
        } catch (_: Exception) {
            emptyList()
        }
    }
}
