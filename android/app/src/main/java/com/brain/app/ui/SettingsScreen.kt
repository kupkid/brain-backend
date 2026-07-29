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
import com.brain.app.ModelInfo
import com.brain.app.ProviderConfig
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(settings: BrainSettings, onBack: () -> Unit, onProviders: () -> Unit = {}) {
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()

    var serverHost by remember { mutableStateOf(settings.serverHost.value) }
    var serverApiKey by remember { mutableStateOf(settings.serverApiKey.value) }
    var providerUrl by remember { mutableStateOf(settings.providerBaseUrl.value) }
    var providerApiKey by remember { mutableStateOf(settings.providerApiKey.value) }
    var llmModel by remember { mutableStateOf(settings.llmModel.value) }
    var embeddingModel by remember { mutableStateOf(settings.embeddingModel.value) }

    var testing by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf<Pair<Boolean, String>?>(null) }
    var fetchingModels by remember { mutableStateOf(false) }
    var models by remember { mutableStateOf<List<ModelInfo>>(emptyList()) }
    var saving by remember { mutableStateOf(false) }
    var saved by remember { mutableStateOf(false) }
    var lastFetchedUrl by remember { mutableStateOf("") }

    LaunchedEffect(providerUrl, providerApiKey) {
        if (providerUrl.isNotBlank() && providerApiKey.isNotBlank() && providerUrl != lastFetchedUrl) {
            delay(1500)
            if (providerUrl.isNotBlank() && providerApiKey.isNotBlank()) {
                fetchingModels = true
                settings.saveProvider(providerUrl, providerApiKey)
                val result = settings.fetchModels()
                models = result.getOrNull() ?: emptyList()
                lastFetchedUrl = providerUrl
                fetchingModels = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = {
                    IconButton(onClick = {
                        settings.saveServer(serverHost, serverApiKey)
                        settings.saveProvider(providerUrl, providerApiKey)
                        settings.saveModels(llmModel, embeddingModel)
                        onBack()
                    }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        },
        containerColor = MaterialTheme.colorScheme.surface
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(scrollState).padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Text("Settings", fontSize = 28.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onSurface)

            // ── Server card ──
            SettingsCard(title = "Server", subtitle = "Brain backend connection") {
                OutlinedTextField(serverHost, { serverHost = it; testResult = null }, Modifier.fillMaxWidth(), label = { Text("Host:Port") }, placeholder = { Text("your-server.com:3000") }, leadingIcon = { Icon(Icons.Default.Dns, null, Modifier.size(20.dp)) }, singleLine = true, shape = RoundedCornerShape(12.dp), keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri))
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(serverApiKey, { serverApiKey = it; testResult = null }, Modifier.fillMaxWidth(), label = { Text("API Key") }, placeholder = { Text("Bearer token") }, leadingIcon = { Icon(Icons.Default.Key, null, Modifier.size(20.dp)) }, visualTransformation = PasswordVisualTransformation(), singleLine = true, shape = RoundedCornerShape(12.dp))
                Spacer(Modifier.height(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilledTonalButton(onClick = {
                        testing = true; testResult = null
                        scope.launch {
                            settings.saveServer(serverHost, serverApiKey)
                            val result = settings.testConnection()
                            testResult = if (result.isSuccess) true to "Connected" else false to (result.exceptionOrNull()?.message ?: "error")
                            testing = false
                        }
                    }, enabled = !testing && serverHost.isNotBlank() && serverApiKey.isNotBlank(), modifier = Modifier.height(36.dp), shape = RoundedCornerShape(10.dp), contentPadding = PaddingValues(horizontal = 16.dp)) {
                        if (testing) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                        else { Icon(Icons.Default.CheckCircle, null, Modifier.size(16.dp)); Spacer(Modifier.width(4.dp)); Text("Test", style = MaterialTheme.typography.labelMedium) }
                    }
                    testResult?.let { (ok, msg) -> Text(msg, color = if (ok) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
                }
            }

            // ── Provider card ──
            SettingsCard(title = "LLM Provider", subtitle = "Configure AI provider") {
                OutlinedTextField(providerUrl, { providerUrl = it; models = emptyList(); lastFetchedUrl = "" }, Modifier.fillMaxWidth(), label = { Text("Base URL") }, placeholder = { Text("https://api.openai.com/v1") }, leadingIcon = { Icon(Icons.Default.Link, null, Modifier.size(20.dp)) }, singleLine = true, shape = RoundedCornerShape(12.dp), keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri))
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(providerApiKey, { providerApiKey = it; models = emptyList(); lastFetchedUrl = "" }, Modifier.fillMaxWidth(), label = { Text("API Key") }, placeholder = { Text("sk-...") }, leadingIcon = { Icon(Icons.Default.VpnKey, null, Modifier.size(20.dp)) }, visualTransformation = PasswordVisualTransformation(), singleLine = true, shape = RoundedCornerShape(12.dp))
                Spacer(Modifier.height(8.dp))

                AnimatedVisibility(visible = fetchingModels, enter = expandVertically(), exit = shrinkVertically()) {
                    Row(Modifier.padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                        Text("Loading models...", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                if (models.isNotEmpty() && !fetchingModels) {
                    Text("${models.size} models available", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }

                ModelDropdown("Chat Model", models, llmModel, { llmModel = it }, "gpt-4o-mini")
                Spacer(Modifier.height(8.dp))
                ModelDropdown("Embedding Model", models, embeddingModel, { embeddingModel = it }, "text-embedding-3-small")

                Spacer(Modifier.height(8.dp))
                OutlinedButton(
                    onClick = onProviders,
                    modifier = Modifier.fillMaxWidth().height(40.dp),
                    shape = RoundedCornerShape(12.dp),
                ) {
                    Icon(Icons.Default.Cloud, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Manage Providers")
                }
            }

            // ── Save button ──
            Button(onClick = {
                saving = true; saved = false
                scope.launch {
                    settings.saveServer(serverHost, serverApiKey)
                    settings.saveProvider(providerUrl, providerApiKey)
                    settings.saveModels(llmModel, embeddingModel)
                    settings.saveProviderConfig(ProviderConfig(base_url = providerUrl, api_key = providerApiKey, llm_model = llmModel, embedding_model = embeddingModel))
                    saving = false; saved = true; delay(2000); saved = false
                }
            }, enabled = !saving && serverHost.isNotBlank() && providerUrl.isNotBlank() && llmModel.isNotBlank() && embeddingModel.isNotBlank(), modifier = Modifier.fillMaxWidth().height(48.dp), shape = RoundedCornerShape(12.dp)) {
                when {
                    saving -> CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp, color = MaterialTheme.colorScheme.onPrimary)
                    saved -> { Icon(Icons.Default.Check, null); Spacer(Modifier.width(6.dp)); Text("Saved") }
                    else -> Text("Save")
                }
            }

            Spacer(Modifier.height(32.dp))
        }
    }
}

@Composable
fun SettingsCard(title: String, subtitle: String, content: @Composable ColumnScope.() -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(title, fontWeight = FontWeight.SemiBold, fontSize = 16.sp, color = MaterialTheme.colorScheme.onSurface)
            Text(subtitle, fontSize = 13.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(12.dp))
            content()
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ModelDropdown(label: String, models: List<ModelInfo>, selected: String, onSelect: (String) -> Unit, placeholder: String) {
    var expanded by remember { mutableStateOf(false) }
    Column {
        OutlinedTextField(
            value = selected, onValueChange = onSelect,
            modifier = Modifier.fillMaxWidth(),
            label = { Text(label) }, placeholder = { Text(placeholder) },
            leadingIcon = { Icon(Icons.Default.SmartToy, null, Modifier.size(20.dp)) },
            trailingIcon = {
                if (models.isNotEmpty()) {
                    IconButton(onClick = { expanded = !expanded }, Modifier.size(32.dp)) {
                        Icon(if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore, null, Modifier.size(20.dp))
                    }
                }
            },
            singleLine = true, shape = RoundedCornerShape(12.dp)
        )
        AnimatedVisibility(visible = expanded && models.isNotEmpty(), enter = expandVertically(), exit = shrinkVertically()) {
            Card(Modifier.fillMaxWidth().padding(top = 4.dp).heightIn(max = 200.dp), RoundedCornerShape(12.dp), CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)) {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    models.forEach { model ->
                        Surface(
                            Modifier.fillMaxWidth().clickable { onSelect(model.id); expanded = false },
                            color = if (model.id == selected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f) else MaterialTheme.colorScheme.surfaceVariant
                        ) {
                            Column(Modifier.padding(horizontal = 14.dp, vertical = 10.dp)) {
                                Text(model.id, style = MaterialTheme.typography.bodyMedium, color = if (model.id == selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                if (model.ownedBy.isNotEmpty()) Text(model.ownedBy, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                }
            }
        }
    }
}
