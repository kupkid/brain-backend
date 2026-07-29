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

        val builder = Request.Builder()
            .url("ws://$host/ws/agent")

        if (apiKey.isNotBlank()) {
            builder.addHeader("Authorization", "Bearer $apiKey")
        }

        val request = builder.build()

        val listener = object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                val msg = Json.encodeToString(
                    TaskRequest.serializer(),
                    TaskRequest(task = task)
                )
                webSocket.send(msg)
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                val event = parseAgentEvent(text)
                trySend(event)
                if (event is DoneEvent || event is ErrorEvent) {
                    webSocket.close(1000, "done")
                }
            }

            override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                onMessage(webSocket, bytes.utf8())
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                webSocket.close(code, reason)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                trySend(ErrorEvent(message = t.message ?: "Connection failed", ts = System.currentTimeMillis() / 1000))
                close()
            }
        }

        val ws = client.newWebSocket(request, listener)

        awaitClose {
            ws.close(1000, "client closed")
        }
    }
}
