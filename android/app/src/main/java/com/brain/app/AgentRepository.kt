package com.brain.app

import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.serialization.json.Json
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
            trySend(ErrorEvent(message = "Server not configured. Open Settings.", ts = System.currentTimeMillis() / 1000))
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
                    trySend(ErrorEvent(message = "Failed to send task: ${e.message}", ts = System.currentTimeMillis() / 1000))
                    close()
                }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val event = parseAgentEvent(text)
                    trySend(event)
                    if (event is DoneEvent || event is ErrorEvent) {
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
                try {
                    trySend(ErrorEvent(
                        message = "Connection failed: ${t.message ?: "unknown"}",
                        ts = System.currentTimeMillis() / 1000
                    ))
                } catch (_: Exception) {}
                try { close() } catch (_: Exception) {}
            }
        }

        val ws = client.newWebSocket(request, listener)
        awaitClose {
            try { ws.close(1000, "client closed") } catch (_: Exception) {}
        }
    }
}
