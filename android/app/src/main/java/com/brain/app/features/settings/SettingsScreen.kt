package com.brain.app.features.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.draw.clip
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.data.BrainSettings
import com.brain.app.theme.BrainColors
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    settings: BrainSettings,
    onBack: () -> Unit,
    onProviders: () -> Unit
) {
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()

    var serverHost by remember { mutableStateOf(settings.serverHost.value) }
    var serverApiKey by remember { mutableStateOf(settings.serverApiKey.value) }
    var testing by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf<Pair<Boolean, String>?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings", color = BrainColors.text100) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back", tint = BrainColors.text200)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = BrainColors.bg000)
            )
        },
        containerColor = BrainColors.bg000
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(scrollState)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp)
        ) {
            // Section: General
            SectionHeader("General")

            SettingsItem(
                icon = Icons.Default.Dns,
                iconColor = BrainColors.accentMain100,
                title = "Server",
                subtitle = serverHost.ifBlank { "Not configured" },
                onClick = { }
            )

            SettingsItem(
                icon = Icons.Default.Key,
                iconColor = BrainColors.warning100,
                title = "API Key",
                subtitle = if (serverApiKey.isNotBlank()) "●●●●●●●●" else "Not set",
                onClick = { }
            )

            // Test connection
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedButton(
                    onClick = {
                        testing = true
                        testResult = null
                        scope.launch {
                            settings.saveServer(serverHost, serverApiKey)
                            val result = settings.testConnection()
                            testResult = if (result.isSuccess) true to "Connected"
                            else false to (result.exceptionOrNull()?.message ?: "Error")
                            testing = false
                        }
                    },
                    enabled = !testing && serverHost.isNotBlank(),
                    modifier = Modifier.height(36.dp),
                    shape = RoundedCornerShape(10.dp),
                    colors = ButtonDefaults.outlinedButtonColors(contentColor = BrainColors.accentMain100)
                ) {
                    if (testing) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp, color = BrainColors.accentMain100)
                    else Text("Test Connection", fontSize = 13.sp)
                }
                testResult?.let { (ok, msg) ->
                    Text(
                        msg,
                        color = if (ok) BrainColors.success100 else BrainColors.danger100,
                        fontSize = 12.sp
                    )
                }
            }

            // Section: Models & Services
            SectionHeader("Models & Services")

            SettingsItem(
                icon = Icons.Default.SmartToy,
                iconColor = BrainColors.accentSecondary100,
                title = "Providers",
                subtitle = "Manage LLM providers",
                onClick = onProviders
            )

            SettingsItem(
                icon = Icons.Default.Storage,
                iconColor = BrainColors.info100,
                title = "Local Data",
                subtitle = "Chats, vault, embeddings",
                onClick = { }
            )

            Spacer(Modifier.height(32.dp))
        }
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(
        text = title,
        color = BrainColors.text300,
        fontSize = 12.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.5.sp
    )
}

@Composable
private fun SettingsItem(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    iconColor: androidx.compose.ui.graphics.Color,
    title: String,
    subtitle: String,
    onClick: () -> Unit
) {
    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(12.dp),
        color = BrainColors.bg300
    ) {
        Row(
            modifier = Modifier.padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Box(
                modifier = Modifier
                    .size(36.dp)
                    .clip(RoundedCornerShape(10.dp))
                    .background(iconColor.copy(alpha = 0.15f)),
                contentAlignment = Alignment.Center
            ) {
                Icon(icon, null, Modifier.size(18.dp), tint = iconColor)
            }

            Column(modifier = Modifier.weight(1f)) {
                Text(title, color = BrainColors.text100, fontSize = 14.sp, fontWeight = FontWeight.Medium)
                Text(subtitle, color = BrainColors.text400, fontSize = 12.sp)
            }

            Icon(
                Icons.Default.ChevronRight,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = BrainColors.text400
            )
        }
    }
}
