package com.brain.app.data

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.mutableStateOf
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

class BrainSettings(context: Context) {
    private val prefs: SharedPreferences = context.getSharedPreferences("brain_settings", Context.MODE_PRIVATE)

    val serverHost = mutableStateOf(prefs.getString("server_host", "") ?: "")
    val serverApiKey = mutableStateOf(prefs.getString("server_api_key", "") ?: "")
    val providerBaseUrl = mutableStateOf(prefs.getString("provider_base_url", "") ?: "")
    val providerApiKey = mutableStateOf(prefs.getString("provider_api_key", "") ?: "")
    val llmModel = mutableStateOf(prefs.getString("llm_model", "") ?: "")
    val embeddingModel = mutableStateOf(prefs.getString("embedding_model", "") ?: "")

    fun saveServer(host: String, apiKey: String) {
        prefs.edit().putString("server_host", host).putString("server_api_key", apiKey).apply()
        serverHost.value = host
        serverApiKey.value = apiKey
    }

    fun saveProvider(url: String, apiKey: String) {
        prefs.edit().putString("provider_base_url", url).putString("provider_api_key", apiKey).apply()
        providerBaseUrl.value = url
        providerApiKey.value = apiKey
    }

    fun saveModels(llm: String, embedding: String) {
        prefs.edit().putString("llm_model", llm).putString("embedding_model", embedding).apply()
        llmModel.value = llm
        embeddingModel.value = embedding
    }

    val isConfigured: Boolean
        get() = serverHost.value.isNotBlank() && serverApiKey.value.isNotBlank()

    fun serverUrl(): String = "http://${serverHost.value}"

    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    suspend fun testConnection(): Result<String> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("${serverUrl()}/health")
                .get()
                .build()
            val response = client.newCall(request).execute()
            if (response.isSuccessful) Result.success("Connected")
            else Result.failure(Exception("HTTP ${response.code}"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun fetchModels(): Result<List<String>> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("${serverUrl()}/v1/providers")
                .addHeader("Authorization", "Bearer ${serverApiKey.value}")
                .get()
                .build()
            val response = client.newCall(request).execute()
            val body = response.body?.string() ?: "[]"
            val arr = org.json.JSONArray(body)
            val models = mutableListOf<String>()
            for (i in 0 until arr.length()) {
                val obj = arr.getJSONObject(i)
                val id = obj.getLong("id")
                val modelsReq = Request.Builder()
                    .url("${serverUrl()}/v1/providers/$id/models")
                    .addHeader("Authorization", "Bearer ${serverApiKey.value}")
                    .get()
                    .build()
                val modelsResp = client.newCall(modelsReq).execute()
                val modelsBody = modelsResp.body?.string() ?: "[]"
                val modelsArr = org.json.JSONArray(modelsBody)
                for (j in 0 until modelsArr.length()) {
                    val model = modelsArr.getJSONObject(j)
                    models.add(model.getString("model_id"))
                }
            }
            Result.success(models)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
