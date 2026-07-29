package com.brain.app.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelEditorScreen(
    settings: BrainSettings,
    providerId: Long,
    modelId: String,
    onBack: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var selectedTab by remember { mutableIntStateOf(0) }
    val tabs = listOf("Basic", "Advanced", "Tools")

    var displayName by remember { mutableStateOf("") }
    var modelType by remember { mutableStateOf("chat") }
    var contextWindow by remember { mutableStateOf("") }
    var maxOutput by remember { mutableStateOf("") }
    var supportsTools by remember { mutableStateOf(false) }
    var supportsVision by remember { mutableStateOf(false) }
    var supportsReasoning by remember { mutableStateOf(false) }
    var supportsAudio by remember { mutableStateOf(false) }
    var supportsVideo by remember { mutableStateOf(false) }
    var inputModalities by remember { mutableStateOf("text") }
    var outputModalities by remember { mutableStateOf("text") }
    var loaded by remember { mutableStateOf(false) }
    var saving by remember { mutableStateOf(false) }
    var saved by remember { mutableStateOf(false) }

    fun loadModel() {
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$providerId/models")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .get()
                    .build()
                val response = editorClient.newCall(request).execute()
                val text = response.body?.string() ?: "[]"
                val arr = org.json.JSONArray(text)
                for (i in 0 until arr.length()) {
                    val obj = arr.getJSONObject(i)
                    if (obj.getString("model_id") == modelId) {
                        displayName = obj.optString("display_name", "")
                        modelType = obj.optString("model_type", "chat")
                        contextWindow = obj.optLong("context_window", 0).let { if (it > 0) it.toString() else "" }
                        maxOutput = obj.optLong("max_output", 0).let { if (it > 0) it.toString() else "" }
                        supportsTools = obj.optInt("supports_tools", 0) == 1
                        supportsVision = obj.optInt("supports_vision", 0) == 1
                        supportsReasoning = obj.optInt("supports_reasoning", 0) == 1
                        supportsAudio = obj.optInt("supports_audio", 0) == 1
                        supportsVideo = obj.optInt("supports_video", 0) == 1
                        inputModalities = obj.optString("input_modalities", "[\"text\"]").removeSurrounding("[\"").removeSurrounding("\"]").replace("\",\"", ", ")
                        outputModalities = obj.optString("output_modalities", "[\"text\"]").removeSurrounding("[\"").removeSurrounding("\"]").replace("\",\"", ", ")
                        loaded = true
                        break
                    }
                }
            } catch (_: Exception) {}
        }
    }

    fun saveModel() {
        saving = true
        scope.launch {
            try {
                val inputArray = inputModalities.split(",").map { "\"${it.trim()}\"" }.joinToString(",", "[", "]")
                val outputArray = outputModalities.split(",").map { "\"${it.trim()}\"" }.joinToString(",", "[", "]")
                val body = JSONObject().apply {
                    put("model_id", modelId)
                    put("model_type", modelType)
                    put("display_name", displayName.ifBlank { null })
                    put("context_window", contextWindow.toLongOrNull())
                    put("max_output", maxOutput.toLongOrNull())
                    put("supports_tools", supportsTools)
                    put("supports_vision", supportsVision)
                    put("supports_reasoning", supportsReasoning)
                    put("supports_audio", supportsAudio)
                    put("supports_video", supportsVideo)
                    put("input_modalities", inputArray)
                    put("output_modalities", outputArray)
                }
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$providerId/models")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .post(body.toString().toRequestBody("application/json".toMediaType()))
                    .build()
                editorClient.newCall(request).execute()
                saving = false
                saved = true
                kotlinx.coroutines.delay(2000)
                saved = false
            } catch (e: Exception) {
                saving = false
            }
        }
    }

    LaunchedEffect(modelId) { loadModel() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(modelId, fontWeight = FontWeight.SemiBold, maxLines = 1, fontSize = 16.sp)
                        Text("Model Editor", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        },
        bottomBar = {
            Surface(color = MaterialTheme.colorScheme.surface, shadowElevation = 8.dp) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    OutlinedButton(
                        onClick = onBack,
                        modifier = Modifier.weight(1f).height(44.dp),
                        shape = RoundedCornerShape(12.dp),
                    ) { Text("Cancel") }
                    Button(
                        onClick = { saveModel() },
                        modifier = Modifier.weight(1f).height(44.dp),
                        shape = RoundedCornerShape(12.dp),
                        enabled = !saving && !saved,
                    ) {
                        when {
                            saving -> CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp, color = MaterialTheme.colorScheme.onPrimary)
                            saved -> { Icon(Icons.Default.Check, null, Modifier.size(18.dp)); Spacer(Modifier.width(6.dp)); Text("Saved") }
                            else -> Text("Save")
                        }
                    }
                }
            }
        },
        containerColor = MaterialTheme.colorScheme.surface,
    ) { padding ->
        Column(modifier = Modifier.padding(padding)) {
            // Tab row
            TabRow(
                selectedTabIndex = selectedTab,
                containerColor = MaterialTheme.colorScheme.surface,
                contentColor = MaterialTheme.colorScheme.primary,
            ) {
                tabs.forEachIndexed { index, title ->
                    Tab(
                        selected = selectedTab == index,
                        onClick = { selectedTab = index },
                        text = {
                            Text(title, fontSize = 13.sp, fontWeight = if (selectedTab == index) FontWeight.SemiBold else FontWeight.Normal)
                        },
                    )
                }
            }

            // Tab content
            Column(
                modifier = Modifier
                    .weight(1f)
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                when (selectedTab) {
                    0 -> BasicTab(displayName, { displayName = it }, modelType, { modelType = it })
                    1 -> AdvancedTab(
                        contextWindow, { contextWindow = it },
                        maxOutput, { maxOutput = it },
                        inputModalities, { inputModalities = it },
                        outputModalities, { outputModalities = it },
                    )
                    2 -> ToolsTab(
                        supportsTools, { supportsTools = it },
                        supportsVision, { supportsVision = it },
                        supportsReasoning, { supportsReasoning = it },
                        supportsAudio, { supportsAudio = it },
                        supportsVideo, { supportsVideo = it },
                    )
                }
            }
        }
    }
}

@Composable
private fun BasicTab(
    displayName: String,
    onDisplayNameChange: (String) -> Unit,
    modelType: String,
    onModelTypeChange: (String) -> Unit,
) {
    val types = listOf("chat", "embedding", "image", "audio", "video")

    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Model Information", fontWeight = FontWeight.SemiBold, fontSize = 15.sp)

        OutlinedTextField(
            value = displayName,
            onValueChange = onDisplayNameChange,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Display Name") },
            placeholder = { Text("GPT-4o Mini") },
            singleLine = true,
            shape = RoundedCornerShape(12.dp),
        )

        Text("Model Type", fontSize = 13.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            types.forEach { type ->
                val selected = modelType == type
                FilterChip(
                    selected = selected,
                    onClick = { onModelTypeChange(type) },
                    label = { Text(type.replaceFirstChar { it.uppercase() }, fontSize = 12.sp) },
                    shape = RoundedCornerShape(10.dp),
                    leadingIcon = if (selected) {
                        { Icon(Icons.Default.Check, null, Modifier.size(16.dp)) }
                    } else null,
                )
            }
        }
    }
}

@Composable
private fun AdvancedTab(
    contextWindow: String,
    onContextWindowChange: (String) -> Unit,
    maxOutput: String,
    onMaxOutputChange: (String) -> Unit,
    inputModalities: String,
    onInputChange: (String) -> Unit,
    outputModalities: String,
    onOutputChange: (String) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Token Limits", fontWeight = FontWeight.SemiBold, fontSize = 15.sp)

        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedTextField(
                value = contextWindow,
                onValueChange = onContextWindowChange,
                modifier = Modifier.weight(1f),
                label = { Text("Context Window") },
                placeholder = { Text("128000") },
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
            )
            OutlinedTextField(
                value = maxOutput,
                onValueChange = onMaxOutputChange,
                modifier = Modifier.weight(1f),
                label = { Text("Max Output") },
                placeholder = { Text("16384") },
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
            )
        }

        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))

        Text("Modalities", fontWeight = FontWeight.SemiBold, fontSize = 15.sp)

        OutlinedTextField(
            value = inputModalities,
            onValueChange = onInputChange,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Input Modalities") },
            placeholder = { Text("text, image") },
            singleLine = true,
            shape = RoundedCornerShape(12.dp),
        )
        OutlinedTextField(
            value = outputModalities,
            onValueChange = onOutputChange,
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Output Modalities") },
            placeholder = { Text("text") },
            singleLine = true,
            shape = RoundedCornerShape(12.dp),
        )

        Text(
            "Comma-separated: text, image, audio, video",
            fontSize = 12.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
        )
    }
}

@Composable
private fun ToolsTab(
    supportsTools: Boolean,
    onToolsChange: (Boolean) -> Unit,
    supportsVision: Boolean,
    onVisionChange: (Boolean) -> Unit,
    supportsReasoning: Boolean,
    onReasoningChange: (Boolean) -> Unit,
    supportsAudio: Boolean,
    onAudioChange: (Boolean) -> Unit,
    supportsVideo: Boolean,
    onVideoChange: (Boolean) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Capabilities", fontWeight = FontWeight.SemiBold, fontSize = 15.sp)

        CapabilityToggle(
            label = "Tool Calling",
            description = "Can use tools, execute functions, call APIs",
            icon = Icons.Default.Build,
            checked = supportsTools,
            onCheckedChange = onToolsChange,
            color = MaterialTheme.colorScheme.primary,
        )
        CapabilityToggle(
            label = "Vision",
            description = "Can process images and visual content",
            icon = Icons.Default.Visibility,
            checked = supportsVision,
            onCheckedChange = onVisionChange,
            color = MaterialTheme.colorScheme.tertiary,
        )
        CapabilityToggle(
            label = "Reasoning",
            description = "Chain-of-thought, deep analysis, math",
            icon = Icons.Default.Psychology,
            checked = supportsReasoning,
            onCheckedChange = onReasoningChange,
            color = MaterialTheme.colorScheme.secondary,
        )
        CapabilityToggle(
            label = "Audio",
            description = "Can process or generate audio",
            icon = Icons.Default.Mic,
            checked = supportsAudio,
            onCheckedChange = onAudioChange,
            color = MaterialTheme.colorScheme.error,
        )
        CapabilityToggle(
            label = "Video",
            description = "Can process or generate video",
            icon = Icons.Default.Videocam,
            checked = supportsVideo,
            onCheckedChange = onVideoChange,
            color = MaterialTheme.colorScheme.error,
        )
    }
}

@Composable
private fun CapabilityToggle(
    label: String,
    description: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    color: androidx.compose.ui.graphics.Color,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (checked) color.copy(alpha = 0.06f)
            else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
        ),
    ) {
        Row(
            modifier = Modifier.padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(icon, null, tint = if (checked) color else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                modifier = Modifier.size(24.dp))
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text(label, fontWeight = FontWeight.Medium, fontSize = 14.sp)
                Text(description, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(
                checked = checked,
                onCheckedChange = onCheckedChange,
                colors = SwitchDefaults.colors(checkedTrackColor = color),
            )
        }
    }
}

private val editorClient = okhttp3.OkHttpClient.Builder()
    .connectTimeout(10, java.util.concurrent.TimeUnit.SECONDS)
    .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
    .build()
