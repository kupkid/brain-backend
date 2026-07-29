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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import com.brain.app.ModelInfo
import com.brain.app.ProviderConfig
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    settings: BrainSettings,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    val focusManager = LocalFocusManager.current
    val scrollState = rememberScrollState()

    var serverHost by remember { mutableStateOf(settings.serverHost) }
    var serverApiKey by remember { mutableStateOf(settings.serverApiKey) }
    var providerUrl by remember { mutableStateOf(settings.providerBaseUrl) }
    var providerApiKey by remember { mutableStateOf(settings.providerApiKey) }
    var llmModel by remember { mutableStateOf(settings.llmModel) }
    var embeddingModel by remember { mutableStateOf(settings.embeddingModel) }

    var testing by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf<Pair<Boolean, String>?>(null) }
    var models by remember { mutableStateOf<List<ModelInfo>>(emptyList()) }
    var fetchingModels by remember { mutableStateOf(false) }
    var saving by remember { mutableStateOf(false) }
    var saved by remember { mutableStateOf(false) }
    var modelsFetchedForUrl by remember { mutableStateOf("") }

    // Auto-fetch models when providerUrl + providerApiKey are filled
    LaunchedEffect(providerUrl, providerApiKey) {
        if (providerUrl.isNotBlank() && providerApiKey.isNotBlank()
            && providerUrl != modelsFetchedForUrl
        ) {
            delay(1500) // debounce
            if (providerUrl.isNotBlank() && providerApiKey.isNotBlank()) {
                fetchingModels = true
                settings.providerBaseUrl = providerUrl
                settings.providerApiKey = providerApiKey
                val result = settings.fetchModels()
                models = result.getOrNull() ?: emptyList()
                modelsFetchedForUrl = providerUrl
                fetchingModels = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings", fontWeight = FontWeight.Medium) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.surface
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(scrollState)
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            // ── Server ──
            Text(
                "Server",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                fontWeight = FontWeight.SemiBold
            )

            OutlinedTextField(
                value = serverHost,
                onValueChange = { serverHost = it; testResult = null },
                modifier = Modifier.fillMaxWidth(),
                label = { Text("Host:Port") },
                placeholder = { Text("148.253.209.232:3000") },
                leadingIcon = { Icon(Icons.Default.Dns, null, modifier = Modifier.size(20.dp)) },
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    unfocusedContainerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                ),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
            )

            OutlinedTextField(
                value = serverApiKey,
                onValueChange = { serverApiKey = it; testResult = null },
                modifier = Modifier.fillMaxWidth(),
                label = { Text("API Key") },
                placeholder = { Text("Bearer token") },
                leadingIcon = { Icon(Icons.Default.Key, null, modifier = Modifier.size(20.dp)) },
                visualTransformation = PasswordVisualTransformation(),
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    unfocusedContainerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                ),
            )

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                FilledTonalButton(
                    onClick = {
                        testing = true
                        testResult = null
                        scope.launch {
                            settings.serverHost = serverHost
                            settings.serverApiKey = serverApiKey
                            val result = settings.testConnection()
                            testResult = if (result.isSuccess) true to "Connected" else false to (result.exceptionOrNull()?.message ?: "error")
                            testing = false
                        }
                    },
                    enabled = !testing && serverHost.isNotBlank() && serverApiKey.isNotBlank(),
                    modifier = Modifier.height(36.dp),
                    shape = RoundedCornerShape(10.dp),
                    contentPadding = PaddingValues(horizontal = 16.dp)
                ) {
                    if (testing) {
                        CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                    } else {
                        Icon(Icons.Default.CheckCircle, null, modifier = Modifier.size(16.dp))
                        Spacer(Modifier.width(4.dp))
                        Text("Test", style = MaterialTheme.typography.labelMedium)
                    }
                }

                testResult?.let { (ok, msg) ->
                    Text(
                        text = msg,
                        color = if (ok) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }

            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

            // ── Provider ──
            Text(
                "LLM Provider",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                fontWeight = FontWeight.SemiBold
            )

            OutlinedTextField(
                value = providerUrl,
                onValueChange = { providerUrl = it; models = emptyList(); modelsFetchedForUrl = "" },
                modifier = Modifier.fillMaxWidth(),
                label = { Text("Base URL") },
                placeholder = { Text("https://api.openai.com/v1") },
                leadingIcon = { Icon(Icons.Default.Link, null, modifier = Modifier.size(20.dp)) },
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    unfocusedContainerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                ),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
            )

            OutlinedTextField(
                value = providerApiKey,
                onValueChange = { providerApiKey = it; models = emptyList(); modelsFetchedForUrl = "" },
                modifier = Modifier.fillMaxWidth(),
                label = { Text("API Key") },
                placeholder = { Text("sk-...") },
                leadingIcon = { Icon(Icons.Default.VpnKey, null, modifier = Modifier.size(20.dp)) },
                visualTransformation = PasswordVisualTransformation(),
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    unfocusedContainerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                ),
            )

            // Models loading
            AnimatedVisibility(
                visible = fetchingModels,
                enter = expandVertically(),
                exit = shrinkVertically()
            ) {
                Row(
                    modifier = Modifier.padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                    Text("Fetching models...", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }

            // Model count
            if (models.isNotEmpty() && !fetchingModels) {
                Text(
                    "${models.size} models available",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            // LLM Model
            ModelSelector(
                label = "Chat Model",
                models = models,
                selected = llmModel,
                onSelect = { llmModel = it },
                placeholder = "gpt-4o-mini"
            )

            // Embedding Model
            ModelSelector(
                label = "Embedding Model",
                models = models,
                selected = embeddingModel,
                onSelect = { embeddingModel = it },
                placeholder = "text-embedding-3-small"
            )

            Spacer(Modifier.height(4.dp))

            // Save button
            Button(
                onClick = {
                    saving = true
                    saved = false
                    scope.launch {
                        settings.serverHost = serverHost
                        settings.serverApiKey = serverApiKey
                        settings.providerBaseUrl = providerUrl
                        settings.providerApiKey = providerApiKey
                        settings.llmModel = llmModel
                        settings.embeddingModel = embeddingModel
                        settings.saveProviderConfig(ProviderConfig(
                            base_url = providerUrl,
                            api_key = providerApiKey,
                            llm_model = llmModel,
                            embedding_model = embeddingModel,
                        ))
                        saving = false
                        saved = true
                        delay(2000)
                        saved = false
                    }
                },
                enabled = !saving && serverHost.isNotBlank() && providerUrl.isNotBlank()
                        && llmModel.isNotBlank() && embeddingModel.isNotBlank(),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
                shape = RoundedCornerShape(12.dp)
            ) {
                if (saving) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp, color = MaterialTheme.colorScheme.onPrimary)
                } else if (saved) {
                    Icon(Icons.Default.Check, null)
                    Spacer(Modifier.width(6.dp))
                    Text("Saved")
                } else {
                    Text("Save")
                }
            }

            Spacer(Modifier.height(32.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ModelSelector(
    label: String,
    models: List<ModelInfo>,
    selected: String,
    onSelect: (String) -> Unit,
    placeholder: String
) {
    var expanded by remember { mutableStateOf(false) }

    Column {
        OutlinedTextField(
            value = selected,
            onValueChange = onSelect,
            modifier = Modifier.fillMaxWidth(),
            label = { Text(label) },
            placeholder = { Text(placeholder) },
            leadingIcon = { Icon(Icons.Default.SmartToy, null, modifier = Modifier.size(20.dp)) },
            trailingIcon = {
                if (models.isNotEmpty()) {
                    IconButton(onClick = { expanded = !expanded }, modifier = Modifier.size(32.dp)) {
                        Icon(
                            if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                            null,
                            modifier = Modifier.size(20.dp)
                        )
                    }
                }
            },
            singleLine = true,
            shape = RoundedCornerShape(12.dp),
            colors = OutlinedTextFieldDefaults.colors(
                unfocusedContainerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
            )
        )

        AnimatedVisibility(
            visible = expanded && models.isNotEmpty(),
            enter = expandVertically(),
            exit = shrinkVertically()
        ) {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 4.dp)
                    .heightIn(max = 200.dp),
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant
                )
            ) {
                Column(modifier = Modifier.verticalScroll(rememberScrollState())) {
                    models.forEach { model ->
                        val isSelected = model.id == selected
                        Surface(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    onSelect(model.id)
                                    expanded = false
                                },
                            color = if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f)
                            else MaterialTheme.colorScheme.surfaceVariant
                        ) {
                            Column(modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp)) {
                                Text(
                                    text = model.id,
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = if (isSelected) MaterialTheme.colorScheme.primary
                                    else MaterialTheme.colorScheme.onSurface,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                                if (model.ownedBy.isNotEmpty()) {
                                    Text(
                                        text = model.ownedBy,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
