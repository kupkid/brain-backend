package com.brain.app.features.settings

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.data.BrainSettings
import com.brain.app.theme.BrainColors
import com.brain.app.theme.BrainShapes
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

private data class Provider(
    val id: Long,
    val name: String,
    val type: String,
    val baseUrl: String,
    val enabled: Boolean,
    val isDefault: Boolean,
    val modelCount: Int = 0
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProvidersScreen(
    settings: BrainSettings,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var providers by remember { mutableStateOf<List<Provider>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }
    var showAdd by remember { mutableStateOf(false) }

    fun loadProviders() {
        scope.launch {
            loading = true
            error = null
            try {
                val result = withContext(Dispatchers.IO) {
                    val client = OkHttpClient.Builder()
                        .connectTimeout(10, TimeUnit.SECONDS)
                        .readTimeout(15, TimeUnit.SECONDS)
                        .build()
                    val req = Request.Builder()
                        .url("${settings.serverUrl()}/v1/providers")
                        .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                        .get()
                        .build()
                    val resp = client.newCall(req).execute()
                    if (resp.isSuccessful) {
                        val body = resp.body?.string() ?: "[]"
                        val arr = JSONArray(body)
                        (0 until arr.length()).map { i ->
                            val obj = arr.getJSONObject(i)
                            Provider(
                                id = obj.getLong("id"),
                                name = obj.getString("name"),
                                type = obj.optString("type", "openai"),
                                baseUrl = obj.optString("base_url", ""),
                                enabled = obj.optBoolean("enabled", true),
                                isDefault = obj.optBoolean("is_default", false)
                            )
                        }
                    } else {
                        emptyList()
                    }
                }
                providers = result
            } catch (e: Exception) {
                error = e.message ?: "Load failed"
            }
            loading = false
        }
    }

    LaunchedEffect(Unit) { loadProviders() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Providers", color = BrainColors.text100) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back", tint = BrainColors.text200)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = BrainColors.bg000)
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showAdd = true },
                containerColor = BrainColors.accentMain100,
                contentColor = Color.White,
                shape = RoundedCornerShape(16.dp)
            ) {
                Icon(Icons.Default.Add, "Add Provider")
            }
        },
        containerColor = BrainColors.bg000
    ) { padding ->
        if (loading) {
            Box(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(color = BrainColors.accentMain100)
            }
        } else if (providers.isEmpty()) {
            Box(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Default.CloudOff, null, Modifier.size(48.dp), tint = BrainColors.text500)
                    Spacer(Modifier.height(8.dp))
                    Text("No providers configured", color = BrainColors.text400, fontSize = 14.sp)
                    Text("Tap + to add one", color = BrainColors.text500, fontSize = 12.sp)
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = 16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
                contentPadding = PaddingValues(vertical = 8.dp)
            ) {
                items(providers, key = { it.id }) { provider ->
                    ProviderCard(
                        provider = provider,
                        onDelete = { id ->
                            scope.launch {
                                withContext(Dispatchers.IO) {
                                    val client = OkHttpClient.Builder().build()
                                    val req = Request.Builder()
                                        .url("${settings.serverUrl()}/v1/providers/$id")
                                        .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                                        .delete()
                                        .build()
                                    client.newCall(req).execute()
                                }
                                loadProviders()
                            }
                        }
                    )
                }
            }
        }
    }

    if (showAdd) {
        AddProviderDialog(
            settings = settings,
            onDismiss = { showAdd = false },
            onSaved = {
                showAdd = false
                loadProviders()
            }
        )
    }
}

@Composable
private fun ProviderCard(
    provider: Provider,
    onDelete: (Long) -> Unit
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(14.dp),
        color = if (provider.enabled) BrainColors.bg300 else BrainColors.danger100.copy(alpha = 0.08f)
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                // Letter avatar
                Box(
                    modifier = Modifier
                        .size(40.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(
                            when (provider.type) {
                                "openai" -> BrainColors.accentMain100.copy(alpha = 0.2f)
                                "anthropic" -> BrainColors.warning100.copy(alpha = 0.2f)
                                else -> BrainColors.accentSecondary100.copy(alpha = 0.2f)
                            }
                        ),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = provider.name.first().uppercase(),
                        color = when (provider.type) {
                            "openai" -> BrainColors.accentMain100
                            "anthropic" -> BrainColors.warning100
                            else -> BrainColors.accentSecondary100
                        },
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold
                    )
                }

                Column(modifier = Modifier.weight(1f)) {
                    Text(provider.name, color = BrainColors.text100, fontSize = 14.sp, fontWeight = FontWeight.Medium)
                    Text(provider.baseUrl, color = BrainColors.text400, fontSize = 11.sp, maxLines = 1)
                }

                // Status badge
                Surface(
                    shape = BrainShapes.full,
                    color = if (provider.enabled) BrainColors.success100.copy(alpha = 0.15f)
                    else BrainColors.text500.copy(alpha = 0.1f)
                ) {
                    Text(
                        text = if (provider.enabled) "Active" else "Off",
                        color = if (provider.enabled) BrainColors.success100 else BrainColors.text500,
                        fontSize = 11.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                    )
                }

                if (provider.isDefault) {
                    Surface(
                        shape = BrainShapes.full,
                        color = BrainColors.accentMain100.copy(alpha = 0.15f)
                    ) {
                        Text(
                            "Default",
                            color = BrainColors.accentMain100,
                            fontSize = 11.sp,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                        )
                    }
                }

                // Delete
                IconButton(onClick = { onDelete(provider.id) }, modifier = Modifier.size(32.dp)) {
                    Icon(Icons.Default.Delete, null, tint = BrainColors.danger100, modifier = Modifier.size(18.dp))
                }
            }
        }
    }
}

@Composable
private fun AddProviderDialog(
    settings: BrainSettings,
    onDismiss: () -> Unit,
    onSaved: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var name by remember { mutableStateOf("") }
    var type by remember { mutableStateOf("openai") }
    var baseUrl by remember { mutableStateOf("") }
    var apiKey by remember { mutableStateOf("") }
    var saving by remember { mutableStateOf(false) }

    val presets = listOf(
        Triple("OpenAI", "openai", "https://api.openai.com/v1"),
        Triple("Google Gemini", "openai", "https://generativelanguage.googleapis.com/v1beta/openai"),
        Triple("Claude", "anthropic", "https://api.anthropic.com/v1"),
        Triple("Cohere", "openai", "https://api.cohere.ai/compatibility/v1"),
        Triple("DeepSeek", "openai", "https://api.deepseek.com/v1"),
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = BrainColors.bg200,
        title = { Text("Add Provider", color = BrainColors.text100) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                // Preset chips
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    presets.take(3).forEach { (presetName, _, presetUrl) ->
                        FilterChip(
                            selected = name == presetName,
                            onClick = {
                                name = presetName
                                val p = presets.find { it.first == presetName }
                                if (p != null) {
                                    type = p.second
                                    baseUrl = p.third
                                }
                            },
                            label = { Text(presetName, fontSize = 11.sp) }
                        )
                    }
                }

                OutlinedTextField(name, { name = it }, Modifier.fillMaxWidth(), label = { Text("Name") },
                    shape = RoundedCornerShape(10.dp),
                    colors = OutlinedTextFieldDefaults.colors(focusedBorderColor = BrainColors.accentMain100, unfocusedBorderColor = BrainColors.border200))
                OutlinedTextField(baseUrl, { baseUrl = it }, Modifier.fillMaxWidth(), label = { Text("Base URL") },
                    singleLine = true, shape = RoundedCornerShape(10.dp),
                    colors = OutlinedTextFieldDefaults.colors(focusedBorderColor = BrainColors.accentMain100, unfocusedBorderColor = BrainColors.border200))
                OutlinedTextField(apiKey, { apiKey = it }, Modifier.fillMaxWidth(), label = { Text("API Key") },
                    singleLine = true, visualTransformation = PasswordVisualTransformation(),
                    shape = RoundedCornerShape(10.dp),
                    colors = OutlinedTextFieldDefaults.colors(focusedBorderColor = BrainColors.accentMain100, unfocusedBorderColor = BrainColors.border200))
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    saving = true
                    scope.launch {
                        try {
                            withContext(Dispatchers.IO) {
                                val client = OkHttpClient.Builder().build()
                                val body = JSONObject().apply {
                                    put("name", name)
                                    put("type", type)
                                    put("base_url", baseUrl)
                                    put("api_key", apiKey)
                                    put("enabled", true)
                                    put("is_default", false)
                                }
                                val req = Request.Builder()
                                    .url("${settings.serverUrl()}/v1/providers")
                                    .addHeader("Authorization", "Bearer ${settings.serverApiKey.value}")
                                    .post(body.toString().toRequestBody("application/json".toMediaType()))
                                    .build()
                                client.newCall(req).execute()
                            }
                            onSaved()
                        } catch (_: Exception) {}
                        saving = false
                    }
                },
                enabled = name.isNotBlank() && baseUrl.isNotBlank() && apiKey.isNotBlank() && !saving,
                colors = ButtonDefaults.buttonColors(containerColor = BrainColors.accentMain100),
                shape = RoundedCornerShape(10.dp)
            ) {
                if (saving) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp, color = Color.White)
                else Text("Save")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, colors = ButtonDefaults.textButtonColors(contentColor = BrainColors.text300)) {
                Text("Cancel")
            }
        }
    )
}
