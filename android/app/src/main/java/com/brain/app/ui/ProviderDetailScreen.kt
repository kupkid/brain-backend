package com.brain.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject

data class ModelDetail(
    val id: Long,
    val modelId: String,
    val modelType: String,
    val displayName: String?,
    val contextWindow: Long?,
    val maxOutput: Long?,
    val supportsTools: Boolean,
    val supportsVision: Boolean,
    val supportsReasoning: Boolean,
    val supportsAudio: Boolean,
    val supportsVideo: Boolean,
    val inputModalities: String,
    val outputModalities: String,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProviderDetailScreen(
    settings: BrainSettings,
    providerId: Long,
    providerName: String,
    onBack: () -> Unit,
    onModelEdit: (Long, String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var models by remember { mutableStateOf<List<ModelDetail>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }
    var fetching by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    fun loadModels() {
        loading = true
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$providerId/models")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .get()
                    .build()
                val response = detailClient.newCall(request).execute()
                val text = response.body?.string() ?: "[]"
                val arr = JSONArray(text)
                val list = mutableListOf<ModelDetail>()
                for (i in 0 until arr.length()) {
                    val obj = arr.getJSONObject(i)
                    list.add(
                        ModelDetail(
                            id = obj.getLong("id"),
                            modelId = obj.getString("model_id"),
                            modelType = obj.optString("model_type", "chat"),
                            displayName = obj.optString("display_name", null),
                            contextWindow = obj.optLong("context_window", 0).takeIf { it > 0 },
                            maxOutput = obj.optLong("max_output", 0).takeIf { it > 0 },
                            supportsTools = obj.optInt("supports_tools", 0) == 1,
                            supportsVision = obj.optInt("supports_vision", 0) == 1,
                            supportsReasoning = obj.optInt("supports_reasoning", 0) == 1,
                            supportsAudio = obj.optInt("supports_audio", 0) == 1,
                            supportsVideo = obj.optInt("supports_video", 0) == 1,
                            inputModalities = obj.optString("input_modalities", "[\"text\"]"),
                            outputModalities = obj.optString("output_modalities", "[\"text\"]"),
                        )
                    )
                }
                models = list
            } catch (e: Exception) {
                statusMessage = "Failed to load: ${e.message}"
            }
            loading = false
        }
    }

    fun fetchModels() {
        fetching = true
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$providerId/fetch-models")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .post("".toRequestBody("application/json".toMediaType()))
                    .build()
                val response = detailClient.newCall(request).execute()
                val text = response.body?.string() ?: "{}"
                val obj = JSONObject(text)
                val saved = obj.optInt("saved", 0)
                val total = obj.optInt("total_fetched", 0)
                statusMessage = "Fetched $total models, saved $saved"
                fetching = false
                loadModels()
            } catch (e: Exception) {
                statusMessage = "Fetch failed: ${e.message}"
                fetching = false
            }
        }
    }

    fun deleteModel(id: Long) {
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$providerId/models/$id")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .delete()
                    .build()
                detailClient.newCall(request).execute()
                statusMessage = "Model deleted"
                loadModels()
            } catch (e: Exception) {
                statusMessage = "Error: ${e.message}"
            }
        }
    }

    LaunchedEffect(providerId) { loadModels() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(providerName, fontWeight = FontWeight.SemiBold, maxLines = 1)
                        Text("${models.size} models", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                actions = {
                    IconButton(onClick = { fetchModels() }, enabled = !fetching) {
                        if (fetching) {
                            CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                        } else {
                            Icon(Icons.Default.Refresh, "Fetch models")
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        },
        containerColor = MaterialTheme.colorScheme.surface,
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            contentPadding = PaddingValues(vertical = 8.dp),
        ) {
            // Status message
            statusMessage?.let { msg ->
                item {
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f)
                        ),
                    ) {
                        Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Info, null, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
                            Spacer(Modifier.width(8.dp))
                            Text(msg, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
                            IconButton(onClick = { statusMessage = null }, Modifier.size(20.dp)) {
                                Icon(Icons.Default.Close, null, Modifier.size(14.dp))
                            }
                        }
                    }
                }
            }

            if (loading) {
                item {
                    Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator(Modifier.size(32.dp), strokeWidth = 2.dp)
                    }
                }
            }

            // Group models by type
            val grouped = models.groupBy { it.modelType }
            val typeOrder = listOf("chat", "embedding", "image", "audio", "video")
            val orderedGroups = typeOrder.mapNotNull { type -> grouped[type]?.let { type to it } }

            orderedGroups.forEach { (type, typeModels) ->
                item {
                    Row(
                        modifier = Modifier.padding(top = 8.dp, bottom = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        ModelTypeIcon(type)
                        Spacer(Modifier.width(8.dp))
                        Text(
                            type.replaceFirstChar { it.uppercase() },
                            fontWeight = FontWeight.SemiBold,
                            fontSize = 14.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(
                            "${typeModels.size}",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                        )
                    }
                }

                items(typeModels, key = { it.id }) { model ->
                    ModelCard(
                        model = model,
                        onClick = { onModelEdit(model.id, model.modelId) },
                        onDelete = { deleteModel(model.id) },
                    )
                }
            }

            if (!loading && models.isEmpty()) {
                item {
                    Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Icon(Icons.Default.CloudDownload, null, Modifier.size(48.dp),
                                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f))
                            Spacer(Modifier.height(8.dp))
                            Text("No models found", color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("Tap refresh to fetch models from provider",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Spacer(Modifier.height(16.dp))
                            FilledTonalButton(onClick = { fetchModels() }) {
                                Icon(Icons.Default.Refresh, null, Modifier.size(16.dp))
                                Spacer(Modifier.width(6.dp))
                                Text("Fetch Models")
                            }
                        }
                    }
                }
            }

            item { Spacer(Modifier.height(80.dp)) }
        }
    }
}

@Composable
private fun ModelCard(
    model: ModelDetail,
    onClick: () -> Unit,
    onDelete: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable { onClick() },
        shape = RoundedCornerShape(14.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)
        ),
    ) {
        Column(Modifier.padding(14.dp)) {
            // Model name + type badge
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(
                        model.displayName ?: model.modelId,
                        fontWeight = FontWeight.Medium,
                        fontSize = 15.sp,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (model.displayName != null && model.displayName != model.modelId) {
                        Text(
                            model.modelId,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            fontSize = 12.sp,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                ModelTypeChip(model.modelType)
            }

            Spacer(Modifier.height(8.dp))

            // Capability badges row
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (model.supportsTools) CapabilityBadge("Tools", MaterialTheme.colorScheme.primary)
                if (model.supportsVision) CapabilityBadge("Vision", MaterialTheme.colorScheme.tertiary)
                if (model.supportsReasoning) CapabilityBadge("Reasoning", MaterialTheme.colorScheme.secondary)
                if (model.supportsAudio) CapabilityBadge("Audio", MaterialTheme.colorScheme.error)
                if (model.supportsVideo) CapabilityBadge("Video", MaterialTheme.colorScheme.error)
                if (!model.supportsTools && !model.supportsVision && !model.supportsReasoning &&
                    !model.supportsAudio && !model.supportsVideo) {
                    Text("--", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f))
                }
            }

            // Context window info
            if (model.contextWindow != null || model.maxOutput != null) {
                Spacer(Modifier.height(6.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    model.contextWindow?.let { ctx ->
                        Text(
                            "Context: ${formatTokenCount(ctx.toInt())}",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                        )
                    }
                    model.maxOutput?.let { out ->
                        Text(
                            "Output: ${formatTokenCount(out.toInt())}",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun CapabilityBadge(label: String, color: androidx.compose.ui.graphics.Color) {
    Surface(
        shape = RoundedCornerShape(6.dp),
        color = color.copy(alpha = 0.12f),
    ) {
        Text(
            label,
            modifier = Modifier.padding(horizontal = 7.dp, vertical = 3.dp),
            fontSize = 10.sp,
            fontWeight = FontWeight.Medium,
            color = color,
        )
    }
}

@Composable
fun ModelTypeIcon(type: String) = when (type) {
    "chat" -> Icon(Icons.Default.Chat, null, Modifier.size(16.dp),
        tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.7f))
    "embedding" -> Icon(Icons.Default.Layers, null, Modifier.size(16.dp),
        tint = MaterialTheme.colorScheme.tertiary.copy(alpha = 0.7f))
    "image" -> Icon(Icons.Default.Image, null, Modifier.size(16.dp),
        tint = MaterialTheme.colorScheme.secondary.copy(alpha = 0.7f))
    "audio" -> Icon(Icons.Default.Mic, null, Modifier.size(16.dp),
        tint = MaterialTheme.colorScheme.error.copy(alpha = 0.7f))
    "video" -> Icon(Icons.Default.Videocam, null, Modifier.size(16.dp),
        tint = MaterialTheme.colorScheme.error.copy(alpha = 0.7f))
    else -> Icon(Icons.Default.HelpOutline, null, Modifier.size(16.dp))
}

@Composable
fun ModelTypeChip(type: String) {
    val color = when (type) {
        "chat" -> MaterialTheme.colorScheme.primary
        "embedding" -> MaterialTheme.colorScheme.tertiary
        "image" -> MaterialTheme.colorScheme.secondary
        "audio", "video" -> MaterialTheme.colorScheme.error
        else -> MaterialTheme.colorScheme.onSurfaceVariant
    }
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = color.copy(alpha = 0.1f),
    ) {
        Text(
            type,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
            fontSize = 10.sp,
            fontWeight = FontWeight.Medium,
            color = color,
        )
    }
}

private fun formatTokenCount(n: Int): String = when {
    n >= 1_000_000 -> "${"%.1f".format(n / 1_000_000f)}M"
    n >= 1_000 -> "${"%.0f".format(n / 1_000f)}K"
    else -> "$n"
}

private val detailClient = okhttp3.OkHttpClient.Builder()
    .connectTimeout(10, java.util.concurrent.TimeUnit.SECONDS)
    .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
    .build()
