package com.brain.app.data

import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.serialization.json.Json
import kotlinx.serialization.encodeToString
import okhttp3.*
import okio.ByteString
import java.util.concurrent.TimeUnit

class AgentRepository(private val settings: BrainSettings) {

    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .pingInterval(30, TimeUnit.SECONDS)
        .build()

    fun connect(task: String): Flow<AgentEvent> = callbackFlow {
        val host = settings.serverHost.value
        val apiKey = settings.serverApiKey.value

        if (host.isBlank()) {
            trySend(AgentEvent.Error(message = "Server not configured. Open Settings."))
            close()
            return@callbackFlow
        }

        val request = Request.Builder()
            .url("ws://$host/ws/agent")
            .apply {
                if (apiKey.isNotBlank()) addHeader("Authorization", "Bearer $apiKey")
            }
            .build()

        val listener = object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                try {
                    val msg = Json.encodeToString(TaskRequest.serializer(), TaskRequest(task = task))
                    webSocket.send(msg)
                } catch (e: Exception) {
                    trySend(AgentEvent.Error(message = "Failed to send task: ${e.message}"))
                    close()
                }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val event = parseAgentEvent(text)
                    trySend(event)
                    if (event is AgentEvent.Done || event is AgentEvent.Error) {
                        webSocket.close(1000, "done")
                    }
                } catch (_: Exception) {}
            }

            override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                onMessage(webSocket, bytes.utf8())
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                try { webSocket.close(code, reason) } catch (_: Exception) {}
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                val msg = when {
                    t.message?.contains("Canceled") == true -> "Connection cancelled"
                    t.message?.contains("timeout") == true -> "Connection timed out"
                    t.message?.contains("refused") == true -> "Server refused connection"
                    t.message?.contains("reset") == true -> "Connection reset"
                    response != null && response.code == 401 -> "Invalid API key"
                    response != null && response.code == 404 -> "Agent endpoint not found"
                    response != null -> "HTTP ${response.code}: ${response.message}"
                    t.message != null -> "Connection error: ${t.message}"
                    else -> "Connection failed"
                }
                try { trySend(AgentEvent.Error(message = msg)) } catch (_: Exception) {}
                try { close() } catch (_: Exception) {}
            }
        }

        val ws = client.newWebSocket(request, listener)
        awaitClose {
            try { ws.close(1000, "client closed") } catch (_: Exception) {}
        }
    }
}
