package com.brain.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import com.brain.app.ModelInfo
import com.brain.app.ProviderConfig
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    settings: BrainSettings,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()

    var serverHost by remember { mutableStateOf(settings.serverHost) }
    var serverApiKey by remember { mutableStateOf(settings.serverApiKey) }
    var providerUrl by remember { mutableStateOf(settings.providerBaseUrl) }
    var providerApiKey by remember { mutableStateOf(settings.providerApiKey) }
    var llmModel by remember { mutableStateOf(settings.llmModel) }
    var embeddingModel by remember { mutableStateOf(settings.embeddingModel) }

    var testing by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf<String?>(null) }
    var fetchingModels by remember { mutableStateOf(false) }
    var models by remember { mutableStateOf<List<ModelInfo>>(emptyList()) }
    var saving by remember { mutableStateOf(false) }
    var saved by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, "Back", tint = Color.White)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color.Black,
                    titleContentColor = Color.White,
                )
            )
        },
        containerColor = Color.Black
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
            contentPadding = PaddingValues(vertical = 16.dp)
        ) {
            // === Server Section ===
            item {
                SectionHeader("Server")
            }

            item {
                SettingsField("Host:Port", serverHost, "148.253.209.232:3000") { serverHost = it }
            }

            item {
                SettingsField("API Key", serverApiKey, "server api key", isPassword = true) { serverApiKey = it }
            }

            item {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Button(
                        onClick = {
                            testing = true
                            testResult = null
                            scope.launch {
                                settings.serverHost = serverHost
                                settings.serverApiKey = serverApiKey
                                val result = settings.testConnection()
                                testResult = result.getOrNull() ?: result.exceptionOrNull()?.message ?: "error"
                                testing = false
                            }
                        },
                        enabled = !testing && serverHost.isNotBlank() && serverApiKey.isNotBlank(),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = Color(0xFF1A1A2E),
                            contentColor = MaterialTheme.colorScheme.primary
                        ),
                        modifier = Modifier.weight(1f)
                    ) {
                        if (testing) {
                            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp, color = Color.White)
                        } else {
                            Text("Test Connection")
                        }
                    }
                }
                if (testResult != null) {
                    Spacer(Modifier.height(4.dp))
                    Text(
                        text = testResult!!,
                        color = if (testResult == "connected") Color(0xFF66BB6A) else Color(0xFFEF5350),
                        fontSize = 12.sp
                    )
                }
            }

            // === Provider Section ===
            item {
                Spacer(Modifier.height(8.dp))
                SectionHeader("LLM Provider (OpenAI-compatible)")
            }

            item {
                SettingsField("Base URL", providerUrl, "https://api.openai.com/v1") { providerUrl = it }
            }

            item {
                SettingsField("API Key", providerApiKey, "provider api key", isPassword = true) { providerApiKey = it }
            }

            item {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Button(
                        onClick = {
                            fetchingModels = true
                            scope.launch {
                                settings.providerBaseUrl = providerUrl
                                settings.providerApiKey = providerApiKey
                                val result = settings.fetchModels()
                                models = result.getOrNull() ?: emptyList()
                                fetchingModels = false
                            }
                        },
                        enabled = !fetchingModels && providerUrl.isNotBlank() && providerApiKey.isNotBlank(),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = Color(0xFF1A1A2E),
                            contentColor = MaterialTheme.colorScheme.primary
                        ),
                        modifier = Modifier.weight(1f)
                    ) {
                        if (fetchingModels) {
                            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp, color = Color.White)
                        } else {
                            Icon(Icons.Default.Refresh, null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text("Fetch Models")
                        }
                    }
                }
            }

            if (models.isNotEmpty()) {
                item {
                    Spacer(Modifier.height(4.dp))
                    Text("Found ${models.size} models", color = Color(0xFF666666), fontSize = 12.sp)
                }
            }

            // LLM Model
            item {
                SettingsField("LLM Model", llmModel, "gpt-4o-mini") { llmModel = it }
            }

            if (models.isNotEmpty()) {
                item {
                    ModelDropdown("LLM Model", models, llmModel) { llmModel = it }
                }
            }

            // Embedding Model
            item {
                SettingsField("Embedding Model", embeddingModel, "text-embedding-3-small") { embeddingModel = it }
            }

            if (models.isNotEmpty()) {
                item {
                    ModelDropdown("Embedding Model", models, embeddingModel) { embeddingModel = it }
                }
            }

            // Save
            item {
                Spacer(Modifier.height(8.dp))
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

                            val config = ProviderConfig(
                                base_url = providerUrl,
                                api_key = providerApiKey,
                                llm_model = llmModel,
                                embedding_model = embeddingModel,
                            )
                            settings.saveProviderConfig(config)
                            saving = false
                            saved = true
                        }
                    },
                    enabled = !saving && serverHost.isNotBlank() && providerUrl.isNotBlank() && llmModel.isNotBlank() && embeddingModel.isNotBlank(),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary,
                        contentColor = Color.White
                    ),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    if (saving) {
                        CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp, color = Color.White)
                    } else if (saved) {
                        Icon(Icons.Default.Check, null)
                        Spacer(Modifier.width(4.dp))
                        Text("Saved")
                    } else {
                        Text("Save & Apply")
                    }
                }
            }
        }
    }
}

@Composable
fun SectionHeader(title: String) {
    Text(
        text = title,
        color = MaterialTheme.colorScheme.primary,
        fontWeight = FontWeight.Bold,
        fontSize = 14.sp,
        letterSpacing = 1.sp
    )
}

@Composable
fun SettingsField(
    label: String,
    value: String,
    placeholder: String,
    isPassword: Boolean = false,
    onValueChange: (String) -> Unit
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label, color = Color(0xFF888888)) },
        placeholder = { Text(placeholder, color = Color(0xFF444444)) },
        modifier = Modifier.fillMaxWidth(),
        visualTransformation = if (isPassword) PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
        colors = OutlinedTextFieldDefaults.colors(
            focusedBorderColor = MaterialTheme.colorScheme.primary,
            unfocusedBorderColor = Color(0xFF333333),
            cursorColor = MaterialTheme.colorScheme.primary,
            focusedTextColor = Color.White,
            unfocusedTextColor = Color.White,
        ),
        singleLine = true
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelDropdown(
    label: String,
    models: List<ModelInfo>,
    selected: String,
    onSelect: (String) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = !expanded }
    ) {
        OutlinedTextField(
            value = selected,
            onValueChange = {},
            readOnly = true,
            label = { Text("$label (from list)", color = Color(0xFF888888)) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .fillMaxWidth()
                .menuAnchor(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                unfocusedBorderColor = Color(0xFF333333),
                cursorColor = MaterialTheme.colorScheme.primary,
                focusedTextColor = Color.White,
                unfocusedTextColor = Color.White,
            )
        )

        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
            containerColor = Color(0xFF1A1A1A)
        ) {
            models.forEach { model ->
                DropdownMenuItem(
                    text = {
                        Column {
                            Text(model.id, color = Color.White, fontSize = 14.sp)
                            if (model.ownedBy.isNotEmpty()) {
                                Text(model.ownedBy, color = Color(0xFF666666), fontSize = 11.sp)
                            }
                        }
                    },
                    onClick = {
                        onSelect(model.id)
                        expanded = false
                    }
                )
            }
        }
    }
}
