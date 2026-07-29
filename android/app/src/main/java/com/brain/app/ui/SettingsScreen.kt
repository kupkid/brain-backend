package com.brain.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.BrainSettings
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(settings: BrainSettings, onBack: () -> Unit, onProviders: () -> Unit = {}) {
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()

    var serverHost by remember { mutableStateOf(settings.serverHost.value) }
    var serverApiKey by remember { mutableStateOf(settings.serverApiKey.value) }
    var testing by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf<Pair<Boolean, String>?>(null) }
    var showServerDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = {
                    IconButton(onClick = {
                        settings.saveServer(serverHost, serverApiKey)
                        onBack()
                    }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent)
            )
        },
        containerColor = Color(0xFF0A0A0A)
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(scrollState)
                .padding(horizontal = 20.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text(
                "Настройки",
                fontSize = 32.sp,
                fontWeight = FontWeight.Bold,
                color = Color.White,
                modifier = Modifier.padding(bottom = 12.dp)
            )

            // ── Section: Общие настройки ──
            SectionHeader("Общие настройки")

            SettingsItem(
                icon = Icons.Default.Palette,
                iconBg = Color(0xFF2A2A2A),
                title = "Тема",
                subtitle = "Тёмная",
                onClick = { /* TODO: theme picker */ }
            )

            SettingsItem(
                icon = Icons.Default.Tune,
                iconBg = Color(0xFF2A2A2A),
                title = "Настройки",
                subtitle = "Сервер, подключение и общие параметры",
                onClick = { showServerDialog = true }
            )

            SettingsItem(
                icon = Icons.Default.SmartToy,
                iconBg = Color(0xFF2A2A2A),
                title = "Ассистент",
                subtitle = "Настроить модель и поведение агента",
                onClick = { /* TODO: agent settings */ }
            )

            Spacer(Modifier.height(8.dp))

            // ── Section: Модели и службы ──
            SectionHeader("Модели и службы")

            SettingsItem(
                icon = Icons.Default.AutoAwesome,
                iconBg = Color(0xFF2A2A2A),
                title = "Модель по умолчанию",
                subtitle = "Установить модель для каждой функции",
                onClick = { /* TODO: default model */ }
            )

            SettingsItem(
                icon = Icons.Default.Cloud,
                iconBg = Color(0xFF1A3A2A),
                title = "Провайдеры",
                subtitle = "Настроить поставщиков ИИ",
                onClick = onProviders
            )

            SettingsItem(
                icon = Icons.Default.Search,
                iconBg = Color(0xFF2A2A2A),
                title = "Служба поиска",
                subtitle = "Настроить службу поиска",
                onClick = { /* TODO */ }
            )

            SettingsItem(
                icon = Icons.Default.RecordVoiceOver,
                iconBg = Color(0xFF2A2A2A),
                title = "Голос",
                subtitle = "Синтез и распознавание речи",
                onClick = { /* TODO */ }
            )

            Spacer(Modifier.height(32.dp))
        }
    }

    // ── Server Config Dialog ──
    if (showServerDialog) {
        ServerConfigDialog(
            host = serverHost,
            apiKey = serverApiKey,
            testing = testing,
            testResult = testResult,
            onHostChange = { serverHost = it; testResult = null },
            onApiKeyChange = { serverApiKey = it; testResult = null },
            onTest = {
                testing = true; testResult = null
                scope.launch {
                    settings.saveServer(serverHost, serverApiKey)
                    val result = settings.testConnection()
                    testResult = if (result.isSuccess) true to "Подключено"
                    else false to (result.exceptionOrNull()?.message ?: "error")
                    testing = false
                }
            },
            onSave = {
                settings.saveServer(serverHost, serverApiKey)
                showServerDialog = false
            },
            onDismiss = { showServerDialog = false }
        )
    }
}

@Composable
fun SectionHeader(title: String) {
    Text(
        title,
        fontSize = 13.sp,
        fontWeight = FontWeight.Medium,
        color = Color(0xFF666666),
        modifier = Modifier.padding(start = 4.dp, top = 8.dp, bottom = 4.dp)
    )
}

@Composable
fun SettingsItem(
    icon: ImageVector,
    iconBg: Color,
    title: String,
    subtitle: String,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = Color(0xFF1A1A1A)),
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier = Modifier
                    .size(40.dp)
                    .clip(CircleShape)
                    .background(iconBg),
                contentAlignment = Alignment.Center
            ) {
                Icon(icon, null, tint = Color(0xFFBBBBBB), modifier = Modifier.size(22.dp))
            }
            Spacer(Modifier.width(14.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(title, color = Color.White, fontSize = 16.sp, fontWeight = FontWeight.Medium)
                Text(subtitle, color = Color(0xFF888888), fontSize = 13.sp, maxLines = 1)
            }
            Icon(
                Icons.Default.ChevronRight, null,
                tint = Color(0xFF444444), modifier = Modifier.size(20.dp)
            )
        }
    }
}

@Composable
fun ServerConfigDialog(
    host: String,
    apiKey: String,
    testing: Boolean,
    testResult: Pair<Boolean, String>?,
    onHostChange: (String) -> Unit,
    onApiKeyChange: (String) -> Unit,
    onTest: () -> Unit,
    onSave: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Сервер", fontWeight = FontWeight.SemiBold) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    host, onHostChange,
                    Modifier.fillMaxWidth(),
                    label = { Text("Host:Port") },
                    placeholder = { Text("your-server.com:3000") },
                    leadingIcon = { Icon(Icons.Default.Dns, null, Modifier.size(20.dp)) },
                    singleLine = true,
                    shape = RoundedCornerShape(12.dp),
                )
                OutlinedTextField(
                    apiKey, onApiKeyChange,
                    Modifier.fillMaxWidth(),
                    label = { Text("API Key") },
                    placeholder = { Text("Bearer token") },
                    leadingIcon = { Icon(Icons.Default.Key, null, Modifier.size(20.dp)) },
                    singleLine = true,
                    shape = RoundedCornerShape(12.dp),
                )
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilledTonalButton(
                        onClick = onTest,
                        enabled = !testing && host.isNotBlank() && apiKey.isNotBlank(),
                        modifier = Modifier.height(36.dp),
                        shape = RoundedCornerShape(10.dp),
                    ) {
                        if (testing) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                        else Text("Тест", style = MaterialTheme.typography.labelMedium)
                    }
                    testResult?.let { (ok, msg) ->
                        Text(msg, color = if (ok) Color(0xFF4CAF50) else MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
        },
        confirmButton = {
            Button(onClick = onSave, enabled = host.isNotBlank() && apiKey.isNotBlank(),
                shape = RoundedCornerShape(10.dp)) {
                Text("Сохранить")
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
