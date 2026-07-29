package com.brain.app

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

@Serializable
data class ProviderConfig(
    val base_url: String = "",
    val api_key: String = "",
    val llm_model: String = "",
    val llm_max_tokens: Int = 8192,
    val embedding_model: String = "",
    val embedding_dimensions: Int = 1024,
)

class BrainSettings(private val context: Context) {
    private val prefs: SharedPreferences = context.getSharedPreferences("brain_settings", Context.MODE_PRIVATE)

    var serverHost: String
        get() = prefs.getString("server_host", "") ?: ""
        set(v) = prefs.edit().putString("server_host", v).apply()

    var serverApiKey: String
        get() = prefs.getString("server_api_key", "") ?: ""
        set(v) = prefs.edit().putString("server_api_key", v).apply()

    var providerBaseUrl: String
        get() = prefs.getString("provider_base_url", "") ?: ""
        set(v) = prefs.edit().putString("provider_base_url", v).apply()

    var providerApiKey: String
        get() = prefs.getString("provider_api_key", "") ?: ""
        set(v) = prefs.edit().putString("provider_api_key", v).apply()

    var llmModel: String
        get() = prefs.getString("llm_model", "") ?: ""
        set(v) = prefs.edit().putString("llm_model", v).apply()

    var embeddingModel: String
        get() = prefs.getString("embedding_model", "") ?: ""
        set(v) = prefs.edit().putString("embedding_model", v).apply()

    val isConfigured: Boolean
        get() = serverHost.isNotBlank() && serverApiKey.isNotBlank()

    fun serverUrl(): String = "http://$serverHost"

    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    private val json = Json { ignoreUnknownKeys = true }

    suspend fun testConnection(): Result<String> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("${serverUrl()}/health")
                .get()
                .build()
            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                Result.success("connected")
            } else {
                Result.failure(Exception("HTTP ${response.code}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun fetchModels(): Result<List<ModelInfo>> = withContext(Dispatchers.IO) {
        try {
            val body = JSONObject().put("path", "/v1/models")
            val request = Request.Builder()
                .url("${serverUrl()}/v1/settings/provider/proxy")
                .addHeader("Authorization", "Bearer $serverApiKey")
                .post(body.toString().toRequestBody("application/json".toMediaType()))
                .build()
            val response = client.newCall(request).execute()
            val text = response.body?.string() ?: "{}"
            val obj = JSONObject(text)
            val data = obj.optJSONArray("data") ?: JSONArray()
            val models = mutableListOf<ModelInfo>()
            for (i in 0 until data.length()) {
                val m = data.getJSONObject(i)
                val owned = m.optString("owned_by", "")
                models.add(ModelInfo(
                    id = m.getString("id"),
                    ownedBy = owned,
                ))
            }
            Result.success(models)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun saveProviderConfig(config: ProviderConfig): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val body = JSONObject().apply {
                put("base_url", config.base_url)
                put("api_key", config.api_key)
                put("llm_model", config.llm_model)
                put("llm_max_tokens", config.llm_max_tokens)
                put("embedding_model", config.embedding_model)
                put("embedding_dimensions", config.embedding_dimensions)
            }
            val request = Request.Builder()
                .url("${serverUrl()}/v1/settings/provider")
                .addHeader("Authorization", "Bearer $serverApiKey")
                .put(body.toString().toRequestBody("application/json".toMediaType()))
                .build()
            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                Result.success(Unit)
            } else {
                Result.failure(Exception("HTTP ${response.code}: ${response.body?.string()}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getProviderStatus(): Result<JSONObject> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("${serverUrl()}/v1/settings/provider")
                .addHeader("Authorization", "Bearer $serverApiKey")
                .get()
                .build()
            val response = client.newCall(request).execute()
            val text = response.body?.string() ?: "{}"
            Result.success(JSONObject(text))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

data class ModelInfo(
    val id: String,
    val ownedBy: String,
)
