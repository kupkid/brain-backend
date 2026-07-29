package com.brain.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

data class ProviderPreset(
    val name: String,
    val type: String,
    val baseUrl: String,
    val icon: String,
)

val PROVIDER_PRESETS = listOf(
    ProviderPreset("OpenAI", "openai", "https://api.openai.com/v1", "🤖"),
    ProviderPreset("Google Gemini", "openai_compat", "https://generativelanguage.googleapis.com/v1beta/openai", "🔮"),
    ProviderPreset("Anthropic Claude", "openai_compat", "https://api.anthropic.com/v1", "🧠"),
    ProviderPreset("Cohere", "cohere", "https://api.cohere.ai/compatibility/v1", "💎"),
    ProviderPreset("DeepSeek", "openai_compat", "https://api.deepseek.com/v1", "🌊"),
    ProviderPreset("Custom", "openai_compat", "", "⚙️"),
)

data class ServerProvider(
    val id: Long,
    val name: String,
    val type: String,
    val baseUrl: String,
    val apiKeySet: Boolean,
    val enabled: Boolean,
    val isDefault: Boolean,
    val models: List<ProviderModel> = emptyList(),
)

data class ProviderModel(
    val modelId: String,
    val modelType: String,
    val displayName: String?,
    val supportsTools: Boolean,
    val supportsVision: Boolean,
    val supportsReasoning: Boolean,
    val inputModalities: String,
    val outputModalities: String,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProvidersScreen(
    settings: BrainSettings,
    onBack: () -> Unit,
    onProviderClick: (Long, String) -> Unit = { _, _ -> },
) {
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()

    var providers by remember { mutableStateOf<List<ServerProvider>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }
    var showAddDialog by remember { mutableStateOf(false) }
    var expandedId by remember { mutableStateOf<Long?>(null) }
    var fetchingModelsId by remember { mutableStateOf<Long?>(null) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    fun loadProviders() {
        loading = true
        scope.launch {
            try {
                val body = JSONObject().put("path", "/v1/providers")
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .get()
                    .build()
                val response = okHttpClient.newCall(request).execute()
                val text = response.body?.string() ?: "[]"
                val arr = JSONArray(text)
                val list = mutableListOf<ServerProvider>()
                for (i in 0 until arr.length()) {
                    val obj = arr.getJSONObject(i)
                    list.add(
                        ServerProvider(
                            id = obj.getLong("id"),
                            name = obj.getString("name"),
                            type = obj.getString("provider_type"),
                            baseUrl = obj.getString("base_url"),
                            apiKeySet = obj.optBoolean("api_key_set", false),
                            enabled = obj.optBoolean("enabled", true),
                            isDefault = obj.optBoolean("is_default", false),
                        )
                    )
                }
                providers = list
            } catch (e: Exception) {
                statusMessage = "Failed to load: ${e.message}"
            }
            loading = false
        }
    }

    fun fetchModels(providerId: Long) {
        fetchingModelsId = providerId
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$providerId/fetch-models")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .post(okhttp3.RequestBody.create(null, byteArrayOf()))
                    .build()
                val response = okHttpClient.newCall(request).execute()
                val text = response.body?.string() ?: "{}"
                val obj = JSONObject(text)
                val saved = obj.optInt("saved", 0)
                val total = obj.optInt("total_fetched", 0)
                statusMessage = "Fetched $total models, saved $saved"
                fetchModelsId = null
                loadProviders()
            } catch (e: Exception) {
                statusMessage = "Fetch failed: ${e.message}"
                fetchingModelsId = null
            }
        }
    }

    fun createProvider(name: String, type: String, baseUrl: String, apiKey: String) {
        scope.launch {
            try {
                val body = JSONObject().apply {
                    put("name", name)
                    put("provider_type", type)
                    put("base_url", baseUrl)
                    put("api_key", apiKey)
                    put("enabled", true)
                }
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .post(body.toString().toRequestBody("application/json".toMediaType()))
                    .build()
                val response = okHttpClient.newCall(request).execute()
                if (response.isSuccessful) {
                    statusMessage = "Provider '$name' created"
                    loadProviders()
                } else {
                    statusMessage = "Error: HTTP ${response.code}"
                }
            } catch (e: Exception) {
                statusMessage = "Error: ${e.message}"
            }
        }
    }

    fun deleteProvider(id: Long) {
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$id")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .delete()
                    .build()
                okHttpClient.newCall(request).execute()
                statusMessage = "Provider deleted"
                loadProviders()
            } catch (e: Exception) {
                statusMessage = "Error: ${e.message}"
            }
        }
    }

    LaunchedEffect(Unit) { loadProviders() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Providers") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                actions = {
                    IconButton(onClick = { showAddDialog = true }) {
                        Icon(Icons.Default.Add, "Add Provider")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = { showAddDialog = true },
                icon = { Icon(Icons.Default.Add, null) },
                text = { Text("Add Provider") },
                containerColor = MaterialTheme.colorScheme.primaryContainer,
            )
        },
        containerColor = MaterialTheme.colorScheme.surface,
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(scrollState)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Spacer(Modifier.height(4.dp))

            // Status message
            statusMessage?.let { msg ->
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f)
                    ),
                ) {
                    Row(
                        Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(Icons.Default.Info, null, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
                        Spacer(Modifier.width(8.dp))
                        Text(msg, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurface)
                        Spacer(Modifier.weight(1f))
                        IconButton(onClick = { statusMessage = null }, Modifier.size(20.dp)) {
                            Icon(Icons.Default.Close, null, Modifier.size(14.dp))
                        }
                    }
                }
            }

            if (loading) {
                Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(Modifier.size(32.dp), strokeWidth = 2.dp)
                }
            }

            // Provider cards
            providers.forEach { provider ->
                ProviderCard(
                    provider = provider,
                    expanded = expandedId == provider.id,
                    onToggleExpand = {
                        expandedId = if (expandedId == provider.id) null else provider.id
                    },
                    onClick = { onProviderClick(provider.id, provider.name) },
                    onFetchModels = { fetchModels(provider.id) },
                    onDelete = { deleteProvider(provider.id) },
                    fetchingModels = fetchingModelsId == provider.id,
                )
            }

            if (!loading && providers.isEmpty()) {
                Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(Icons.Default.CloudOff, null, Modifier.size(48.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f))
                        Spacer(Modifier.height(8.dp))
                        Text("No providers configured", color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("Tap + to add one", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }

            Spacer(Modifier.height(80.dp))
        }
    }

    if (showAddDialog) {
        AddProviderDialog(
            onDismiss = { showAddDialog = false },
            onAdd = { name, type, url, key ->
                createProvider(name, type, url, key)
                showAddDialog = false
            },
        )
    }
}

@Composable
private fun ProviderCard(
    provider: ServerProvider,
    expanded: Boolean,
    onToggleExpand: () -> Unit,
    onClick: () -> Unit,
    onFetchModels: () -> Unit,
    onDelete: () -> Unit,
    fetchingModels: Boolean,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { if (expanded) onClick() else onToggleExpand() },
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (provider.enabled) {
                MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
            } else {
                MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
            }
        ),
    ) {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                // Icon based on type
                val icon = when (provider.type) {
                    "openai" -> Icons.Default.SmartToy
                    "cohere" -> Icons.Default.Diamond
                    else -> Icons.Default.Cloud
                }
                Icon(icon, null, Modifier.size(24.dp), tint = MaterialTheme.colorScheme.primary)
                Spacer(Modifier.width(12.dp))
                Column(Modifier.weight(1f)) {
                    Text(provider.name, fontWeight = FontWeight.SemiBold, fontSize = 16.sp)
                    Text(
                        provider.baseUrl.take(50) + if (provider.baseUrl.length > 50) "..." else "",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                // Badges
                if (provider.isDefault) {
                    AssistChip(
                        onClick = {},
                        label = { Text("Default", fontSize = 10.sp) },
                        modifier = Modifier.height(24.dp),
                        shape = RoundedCornerShape(8.dp),
                    )
                }
                if (!provider.enabled) {
                    AssistChip(
                        onClick = {},
                        label = { Text("Off", fontSize = 10.sp) },
                        modifier = Modifier.height(24.dp),
                        shape = RoundedCornerShape(8.dp),
                        colors = AssistChipDefaults.assistChipColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.5f),
                        ),
                    )
                }
                Spacer(Modifier.width(4.dp))
                Icon(
                    if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    null,
                    Modifier.size(20.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            // Expanded: actions
            AnimatedVisibility(visible = expanded, enter = expandVertically(), exit = shrinkVertically()) {
                Column(Modifier.padding(top = 12.dp)) {
                    HorizontalDivider(Modifier, color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))
                    Spacer(Modifier.height(12.dp))

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        FilledTonalButton(
                            onClick = onFetchModels,
                            enabled = !fetchingModels,
                            modifier = Modifier.height(36.dp),
                            shape = RoundedCornerShape(10.dp),
                            contentPadding = PaddingValues(horizontal = 16.dp),
                        ) {
                            if (fetchingModels) {
                                CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                            } else {
                                Icon(Icons.Default.Refresh, null, Modifier.size(16.dp))
                            }
                            Spacer(Modifier.width(4.dp))
                            Text("Fetch Models", style = MaterialTheme.typography.labelMedium)
                        }
                        FilledTonalButton(
                            onClick = onDelete,
                            modifier = Modifier.height(36.dp),
                            shape = RoundedCornerShape(10.dp),
                            contentPadding = PaddingValues(horizontal = 16.dp),
                            colors = ButtonDefaults.filledTonalButtonColors(
                                containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.5f),
                            ),
                        ) {
                            Icon(Icons.Default.Delete, null, Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text("Delete", style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AddProviderDialog(
    onDismiss: () -> Unit,
    onAdd: (name: String, type: String, baseUrl: String, apiKey: String) -> Unit,
) {
    var selectedPreset by remember { mutableStateOf<ProviderPreset?>(null) }
    var name by remember { mutableStateOf("") }
    var type by remember { mutableStateOf("openai_compat") }
    var baseUrl by remember { mutableStateOf("") }
    var apiKey by remember { mutableStateOf("") }
    var showKey by remember { mutableStateOf(false) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add Provider", fontWeight = FontWeight.SemiBold) },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Choose a preset or configure custom:", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

                // Preset chips
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    PROVIDER_PRESETS.take(3).forEach { preset ->
                        FilterChip(
                            selected = selectedPreset == preset,
                            onClick = {
                                selectedPreset = preset
                                name = preset.name
                                type = preset.type
                                baseUrl = preset.baseUrl
                            },
                            label = { Text("${preset.icon} ${preset.name}", maxLines = 1) },
                            modifier = Modifier.weight(1f),
                            shape = RoundedCornerShape(10.dp),
                        )
                    }
                }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    PROVIDER_PRESETS.drop(3).forEach { preset ->
                        FilterChip(
                            selected = selectedPreset == preset,
                            onClick = {
                                selectedPreset = preset
                                name = preset.name
                                type = preset.type
                                baseUrl = preset.baseUrl
                            },
                            label = { Text("${preset.icon} ${preset.name}", maxLines = 1) },
                            modifier = Modifier.weight(1f),
                            shape = RoundedCornerShape(10.dp),
                        )
                    }
                }

                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))

                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Name") },
                    placeholder = { Text("My Provider") },
                    leadingIcon = { Icon(Icons.Default.Label, null, Modifier.size(20.dp)) },
                    singleLine = true,
                    shape = RoundedCornerShape(12.dp),
                )
                OutlinedTextField(
                    value = baseUrl,
                    onValueChange = { baseUrl = it; selectedPreset = null },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Base URL") },
                    placeholder = { Text("https://api.openai.com/v1") },
                    leadingIcon = { Icon(Icons.Default.Link, null, Modifier.size(20.dp)) },
                    singleLine = true,
                    shape = RoundedCornerShape(12.dp),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                )
                OutlinedTextField(
                    value = apiKey,
                    onValueChange = { apiKey = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("API Key") },
                    placeholder = { Text("sk-...") },
                    leadingIcon = { Icon(Icons.Default.Key, null, Modifier.size(20.dp)) },
                    trailingIcon = {
                        IconButton(onClick = { showKey = !showKey }, Modifier.size(32.dp)) {
                            Icon(
                                if (showKey) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                null,
                                Modifier.size(20.dp),
                            )
                        }
                    },
                    visualTransformation = if (showKey) {
                        androidx.compose.ui.text.input.VisualTransformation.None
                    } else {
                        PasswordVisualTransformation()
                    },
                    singleLine = true,
                    shape = RoundedCornerShape(12.dp),
                )
            }
        },
        confirmButton = {
            Button(
                onClick = { onAdd(name, type, baseUrl, apiKey) },
                enabled = name.isNotBlank() && baseUrl.isNotBlank() && apiKey.isNotBlank(),
                shape = RoundedCornerShape(10.dp),
            ) {
                Icon(Icons.Default.Add, null, Modifier.size(16.dp))
                Spacer(Modifier.width(4.dp))
                Text("Add")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, shape = RoundedCornerShape(10.dp)) {
                Text("Cancel")
            }
        },
        shape = RoundedCornerShape(20.dp),
    )
}

private val okHttpClient = okhttp3.OkHttpClient.Builder()
    .connectTimeout(10, java.util.concurrent.TimeUnit.SECONDS)
    .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
    .build()

private fun String.toRequestBody(mediaType: String): okhttp3.RequestBody =
    okhttp3.RequestBody.create(okhttp3.MediaType.parse(mediaType), this)
