package com.brain.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject

data class ProviderPreset(
    val name: String,
    val type: String,
    val baseUrl: String,
    val letter: String,
    val color: Color,
)

val PROVIDER_PRESETS = listOf(
    ProviderPreset("OpenAI", "openai", "https://api.openai.com/v1", "O", Color(0xFF10A37F)),
    ProviderPreset("Google Gemini", "openai_compat", "https://generativelanguage.googleapis.com/v1beta/openai", "G", Color(0xFF4285F4)),
    ProviderPreset("Anthropic Claude", "openai_compat", "https://api.anthropic.com/v1", "A", Color(0xFFD4A574)),
    ProviderPreset("Cohere", "cohere", "https://api.cohere.ai/compatibility/v1", "C", Color(0xFF39D98A)),
    ProviderPreset("DeepSeek", "openai_compat", "https://api.deepseek.com/v1", "D", Color(0xFF5B7FFF)),
    ProviderPreset("Custom", "openai_compat", "", "C", Color(0xFF888888)),
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
    val modelCount: Int = 0,
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

private val okHttpClient = okhttp3.OkHttpClient.Builder()
    .connectTimeout(10, java.util.concurrent.TimeUnit.SECONDS)
    .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
    .build()

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProvidersScreen(
    settings: BrainSettings,
    onBack: () -> Unit,
    onProviderClick: (Long, String) -> Unit = { _, _ -> },
) {
    val scope = rememberCoroutineScope()

    var providers by remember { mutableStateOf<List<ServerProvider>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }
    var showAddDialog by remember { mutableStateOf(false) }
    var fetchingModelsId by remember { mutableStateOf<Long?>(null) }
    var statusMessage by remember { mutableStateOf<String?>(null) }
    var searchQuery by remember { mutableStateOf("") }
    var menuExpandedId by remember { mutableStateOf<Long?>(null) }

    fun loadProviders() {
        loading = true
        scope.launch {
            try {
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .get()
                    .build()
                val response = okHttpClient.newCall(request).execute()
                if (!response.isSuccessful) {
                    statusMessage = "Ошибка: HTTP ${response.code}"
                    loading = false
                    return@launch
                }
                val body = response.body?.string()
                if (body == null) {
                    statusMessage = "Ошибка: пустой ответ"
                    loading = false
                    return@launch
                }
                val arr = JSONArray(body)
                val list = mutableListOf<ServerProvider>()
                for (i in 0 until arr.length()) {
                    val obj = arr.getJSONObject(i)
                    val pid = obj.getLong("id")
                    // Fetch model count
                    var modelCount = 0
                    try {
                        val mReq = okhttp3.Request.Builder()
                            .url("${settings.serverUrl()}/v1/providers/$pid/models")
                            .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                            .get()
                            .build()
                        val mResp = okHttpClient.newCall(mReq).execute()
                        if (mResp.isSuccessful) {
                            val mBody = mResp.body?.string()
                            if (mBody != null) {
                                val mArr = JSONArray(mBody)
                                modelCount = mArr.length()
                            }
                        }
                    } catch (_: Exception) {}
                    list.add(
                        ServerProvider(
                            id = pid,
                            name = obj.getString("name"),
                            type = obj.optString("provider_type", "openai_compat"),
                            baseUrl = obj.optString("base_url", ""),
                            apiKeySet = obj.optBoolean("api_key_set", false),
                            enabled = obj.optBoolean("enabled", true),
                            isDefault = obj.optBoolean("is_default", false),
                            modelCount = modelCount,
                        )
                    )
                }
                providers = list
            } catch (e: Exception) {
                statusMessage = "Ошибка загрузки: ${e.message}"
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
                    .post(ByteArray(0).toRequestBody(null))
                    .build()
                val response = okHttpClient.newCall(request).execute()
                if (!response.isSuccessful) {
                    statusMessage = "Ошибка: HTTP ${response.code}"
                    fetchingModelsId = null
                    return@launch
                }
                val body = response.body?.string() ?: "{}"
                val obj = JSONObject(body)
                val saved = obj.optInt("saved", 0)
                statusMessage = "Загружено $saved моделей"
                fetchingModelsId = null
                loadProviders()
            } catch (e: Exception) {
                statusMessage = "Ошибка: ${e.message}"
                fetchingModelsId = null
            }
        }
    }

    fun toggleProvider(id: Long, enabled: Boolean) {
        scope.launch {
            try {
                val body = JSONObject().put("enabled", !enabled)
                val request = okhttp3.Request.Builder()
                    .url("${settings.serverUrl()}/v1/providers/$id")
                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                    .put(body.toString().toRequestBody("application/json".toMediaType()))
                    .build()
                okHttpClient.newCall(request).execute()
                loadProviders()
            } catch (e: Exception) {
                statusMessage = "Ошибка: ${e.message}"
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
                statusMessage = "Провайдер удалён"
                loadProviders()
            } catch (e: Exception) {
                statusMessage = "Ошибка: ${e.message}"
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
                    statusMessage = "Провайдер '$name' создан"
                    loadProviders()
                } else {
                    statusMessage = "Ошибка: HTTP ${response.code}"
                }
            } catch (e: Exception) {
                statusMessage = "Ошибка: ${e.message}"
            }
        }
    }

    LaunchedEffect(Unit) { loadProviders() }

    val filteredProviders = providers.filter {
        searchQuery.isBlank() || it.name.contains(searchQuery, ignoreCase = true)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back", tint = Color.White)
                    }
                },
                actions = {
                    IconButton(onClick = { showAddDialog = true }) {
                        Icon(Icons.Default.Add, "Добавить", tint = Color.White)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent)
            )
        },
        containerColor = Color(0xFF0A0A0A),
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 20.dp),
        ) {
            // Title
            Text(
                "Провайдеры",
                fontSize = 32.sp,
                fontWeight = FontWeight.Bold,
                color = Color.White,
                modifier = Modifier.padding(bottom = 16.dp)
            )

            // Search bar
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { searchQuery = it },
                modifier = Modifier.fillMaxWidth(),
                placeholder = { Text("Поиск поставщиков", color = Color(0xFF666666)) },
                leadingIcon = { Icon(Icons.Default.Search, null, tint = Color(0xFF666666), modifier = Modifier.size(20.dp)) },
                singleLine = true,
                shape = RoundedCornerShape(14.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    unfocusedBorderColor = Color(0xFF2A2A2A),
                    focusedBorderColor = Color(0xFF3A3A3A),
                    unfocusedContainerColor = Color(0xFF141414),
                    focusedContainerColor = Color(0xFF141414),
                    cursorColor = Color.White,
                    focusedTextColor = Color.White,
                    unfocusedTextColor = Color.White,
                ),
            )

            Spacer(Modifier.height(12.dp))

            // Status message
            statusMessage?.let { msg ->
                val isError = msg.startsWith("Ошибка")
                Card(
                    modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
                    shape = RoundedCornerShape(10.dp),
                    colors = CardDefaults.cardColors(
                        containerColor = if (isError) Color(0xFF2A1A1A) else Color(0xFF1A2A1A)
                    ),
                ) {
                    Row(Modifier.padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(msg,
                            color = if (isError) Color(0xFFEF5350) else Color(0xFF4CAF50),
                            fontSize = 13.sp, modifier = Modifier.weight(1f))
                        IconButton(onClick = { statusMessage = null }, Modifier.size(20.dp)) {
                            Icon(Icons.Default.Close, null, Modifier.size(14.dp), tint = Color(0xFF666666))
                        }
                    }
                }
            }

            // Loading
            if (loading) {
                Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(Modifier.size(28.dp), strokeWidth = 2.dp, color = Color(0xFF4CAF50))
                }
            }

            // Provider cards
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                filteredProviders.forEach { provider ->
                    ProviderCardModern(
                        provider = provider,
                        onClick = { onProviderClick(provider.id, provider.name) },
                        onFetchModels = { fetchModels(provider.id) },
                        onToggle = { toggleProvider(provider.id, provider.enabled) },
                        onDelete = { deleteProvider(provider.id) },
                        onMenuExpand = { menuExpandedId = if (menuExpandedId == provider.id) null else provider.id },
                        menuExpanded = menuExpandedId == provider.id,
                        fetchingModels = fetchingModelsId == provider.id,
                    )
                }
            }

            // Empty state
            if (!loading && filteredProviders.isEmpty()) {
                Box(Modifier.fillMaxWidth().padding(48.dp), contentAlignment = Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(Icons.Default.CloudOff, null, Modifier.size(48.dp),
                            tint = Color(0xFF333333))
                        Spacer(Modifier.height(12.dp))
                        Text("Нет провайдеров", color = Color(0xFF666666))
                        Text("Нажмите + чтобы добавить", fontSize = 13.sp, color = Color(0xFF444444))
                    }
                }
            }

            Spacer(Modifier.height(16.dp))
        }
    }

    if (showAddDialog) {
        AddProviderDialogModern(
            onDismiss = { showAddDialog = false },
            onAdd = { name, type, url, key ->
                createProvider(name, type, url, key)
                showAddDialog = false
            },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProviderCardModern(
    provider: ServerProvider,
    onClick: () -> Unit,
    onFetchModels: () -> Unit,
    onToggle: () -> Unit,
    onDelete: () -> Unit,
    onMenuExpand: () -> Unit,
    menuExpanded: Boolean,
    fetchingModels: Boolean,
) {
    val cardColor = if (provider.enabled) Color(0xFF1A1A1A) else Color(0xFF2A1515)
    val letterColor = when (provider.name.firstOrNull()?.uppercase()) {
        "O" -> Color(0xFF10A37F)
        "G" -> Color(0xFF4285F4)
        "A" -> Color(0xFFD4A574)
        "C" -> Color(0xFF39D98A)
        "D" -> Color(0xFF5B7FFF)
        "E" -> Color(0xFF9B59B6)
        "M" -> Color(0xFFFF6B6B)
        "S" -> Color(0xFFE74C3C)
        "Y" -> Color(0xFF3498DB)
        else -> Color(0xFF888888)
    }

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .clickable { onClick() },
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = cardColor),
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Letter avatar
            Box(
                modifier = Modifier
                    .size(48.dp)
                    .clip(CircleShape)
                    .background(letterColor.copy(alpha = 0.15f)),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    provider.name.firstOrNull()?.uppercase() ?: "?",
                    color = letterColor,
                    fontSize = 20.sp,
                    fontWeight = FontWeight.Bold
                )
            }

            Spacer(Modifier.width(14.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    provider.name,
                    color = Color.White,
                    fontSize = 17.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(4.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    // Status badge
                    val enabledColor = if (provider.enabled) Color(0xFF2E7D32) else Color(0xFF8B4513)
                    val enabledText = if (provider.enabled) "Включено" else "Отключено"
                    Surface(
                        shape = RoundedCornerShape(6.dp),
                        color = enabledColor.copy(alpha = 0.3f),
                    ) {
                        Text(
                            enabledText,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                            fontSize = 11.sp,
                            color = if (provider.enabled) Color(0xFF66BB6A) else Color(0xFFCD853F),
                            fontWeight = FontWeight.Medium,
                        )
                    }
                    // Model count badge
                    if (provider.modelCount > 0) {
                        Surface(
                            shape = RoundedCornerShape(6.dp),
                            color = Color(0xFF1565C0).copy(alpha = 0.3f),
                        ) {
                            Text(
                                "${provider.modelCount} моделей",
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                                fontSize = 11.sp,
                                color = Color(0xFF64B5F6),
                                fontWeight = FontWeight.Medium,
                            )
                        }
                    }
                }
            }

            // Menu button
            Box {
                IconButton(onClick = onMenuExpand, modifier = Modifier.size(36.dp)) {
                    Icon(Icons.Default.MoreVert, null, tint = Color(0xFF888888), modifier = Modifier.size(20.dp))
                }
                DropdownMenu(
                    expanded = menuExpanded,
                    onDismissRequest = onMenuExpand,
                ) {
                    DropdownMenuItem(
                        text = { Text(if (provider.enabled) "Отключить" else "Включить") },
                        onClick = { onToggle(); onMenuExpand() },
                        leadingIcon = { Icon(if (provider.enabled) Icons.Default.VisibilityOff else Icons.Default.Visibility, null, Modifier.size(18.dp)) },
                    )
                    DropdownMenuItem(
                        text = { Text("Загрузить модели") },
                        onClick = { onFetchModels(); onMenuExpand() },
                        leadingIcon = { Icon(Icons.Default.Refresh, null, Modifier.size(18.dp)) },
                    )
                    DropdownMenuItem(
                        text = { Text("Удалить", color = MaterialTheme.colorScheme.error) },
                        onClick = { onDelete(); onMenuExpand() },
                        leadingIcon = { Icon(Icons.Default.Delete, null, Modifier.size(18.dp), tint = MaterialTheme.colorScheme.error) },
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AddProviderDialogModern(
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
        title = { Text("Добавить поставщика", fontWeight = FontWeight.SemiBold) },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Выберите пресет или настройте вручную:", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

                // Preset chips
                PROVIDER_PRESETS.chunked(3).forEach { row ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        row.forEach { preset ->
                            FilterChip(
                                selected = selectedPreset == preset,
                                onClick = {
                                    selectedPreset = preset
                                    name = preset.name
                                    type = preset.type
                                    baseUrl = preset.baseUrl
                                },
                                label = { Text(preset.name, maxLines = 1, fontSize = 12.sp) },
                                modifier = Modifier.weight(1f),
                                shape = RoundedCornerShape(10.dp),
                            )
                        }
                    }
                }

                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))

                OutlinedTextField(name, { name = it }, Modifier.fillMaxWidth(),
                    label = { Text("Имя") }, placeholder = { Text("My Provider") },
                    leadingIcon = { Icon(Icons.Default.Label, null, Modifier.size(20.dp)) },
                    singleLine = true, shape = RoundedCornerShape(12.dp))
                OutlinedTextField(baseUrl, { baseUrl = it; selectedPreset = null }, Modifier.fillMaxWidth(),
                    label = { Text("Base URL") }, placeholder = { Text("https://api.openai.com/v1") },
                    leadingIcon = { Icon(Icons.Default.Link, null, Modifier.size(20.dp)) },
                    singleLine = true, shape = RoundedCornerShape(12.dp),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri))
                OutlinedTextField(apiKey, { apiKey = it }, Modifier.fillMaxWidth(),
                    label = { Text("API Key") }, placeholder = { Text("sk-...") },
                    leadingIcon = { Icon(Icons.Default.Key, null, Modifier.size(20.dp)) },
                    trailingIcon = {
                        IconButton(onClick = { showKey = !showKey }, Modifier.size(32.dp)) {
                            Icon(if (showKey) Icons.Default.VisibilityOff else Icons.Default.Visibility, null, Modifier.size(20.dp))
                        }
                    },
                    visualTransformation = if (showKey) androidx.compose.ui.text.input.VisualTransformation.None
                    else PasswordVisualTransformation(),
                    singleLine = true, shape = RoundedCornerShape(12.dp))
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
                Text("Добавить")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, shape = RoundedCornerShape(10.dp)) {
                Text("Отмена")
            }
        },
        shape = RoundedCornerShape(20.dp),
    )
}
