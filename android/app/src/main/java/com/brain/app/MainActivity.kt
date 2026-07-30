package com.brain.app

import android.app.Application
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.*
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.brain.app.data.BrainSettings
import com.brain.app.features.chat.*
import com.brain.app.features.settings.SettingsScreen
import com.brain.app.features.settings.ProvidersScreen
import com.brain.app.theme.BrainColors
import com.brain.app.theme.BrainShapes
import com.brain.app.theme.BrainTheme
import com.brain.app.viewmodel.BrainViewModel
import com.brain.app.viewmodel.BrainViewModelFactory

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            BrainTheme {
                BrainApp()
            }
        }
    }
}

@Composable
fun BrainApp() {
    val context = LocalContext.current
    val settings = remember { BrainSettings(context) }
    val factory = remember { BrainViewModelFactory(context.applicationContext as Application, settings) }
    val vm: BrainViewModel = viewModel(factory = factory)

    var showSettings by remember { mutableStateOf(false) }
    var showProviders by remember { mutableStateOf(false) }
    var inputValue by remember { mutableStateOf("") }
    var showModelSelector by remember { mutableStateOf(false) }
    var showDrawer by remember { mutableStateOf(false) }

    val events by vm.events.collectAsState()
    val isRunning by vm.isRunning.collectAsState()
    val selectedModel by vm.selectedModel.collectAsState()

    when {
        showProviders -> ProvidersScreen(
            settings = settings,
            onBack = { showProviders = false }
        )
        showSettings -> SettingsScreen(
            settings = settings,
            onBack = { showSettings = false },
            onProviders = { showProviders = true }
        )
        else -> {
            ModalNavigationDrawer(
                drawerContent = {
                    DrawerContent(
                        onNewChat = { vm.newChat() },
                        onSettings = {
                            showDrawer = false
                            showSettings = true
                        },
                        onDismiss = { showDrawer = false }
                    )
                },
                gesturesEnabled = showDrawer
            ) {
                Scaffold(
                    topBar = {
                        Header(
                            title = "New chat",
                            selectedModel = selectedModel,
                            onTitleChange = { },
                            onModelClick = { showModelSelector = true },
                            onMenuClick = { showDrawer = true }
                        )
                    },
                    containerColor = BrainColors.bg000
                ) { padding ->
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(padding)
                    ) {
                        if (events.isEmpty() && !isRunning) {
                            EmptyState(
                                onStartChat = { task ->
                                    inputValue = ""
                                    vm.sendTask(task)
                                }
                            )
                        } else {
                            Column(modifier = Modifier.fillMaxSize()) {
                                ChatArea(
                                    events = events,
                                    isRunning = isRunning,
                                    streamingText = "",
                                    modifier = Modifier.weight(1f)
                                )
                                InputBox(
                                    value = inputValue,
                                    onValueChange = { inputValue = it },
                                    onSend = {
                                        if (inputValue.isNotBlank()) {
                                            vm.sendTask(inputValue)
                                            inputValue = ""
                                        }
                                    },
                                    onStop = { vm.stopAgent() },
                                    isRunning = isRunning,
                                    selectedModel = selectedModel,
                                    onModelClick = { showModelSelector = true }
                                )
                            }
                        }
                    }
                }
            }

            if (showModelSelector) {
                ModelSelector(
                    models = vm.availableModels.collectAsState().value,
                    selected = selectedModel,
                    onSelect = { model ->
                        vm.selectModel(model)
                        showModelSelector = false
                    },
                    onDismiss = { showModelSelector = false }
                )
            }
        }
    }
}

@Composable
private fun DrawerContent(
    onNewChat: () -> Unit,
    onSettings: () -> Unit,
    onDismiss: () -> Unit
) {
    ModalDrawerSheet(
        modifier = Modifier.width(280.dp),
        drawerContainerColor = BrainColors.bg200
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Brain",
                    color = BrainColors.text100,
                    fontSize = 20.sp,
                    fontWeight = FontWeight.Bold
                )
                IconButton(onClick = onDismiss, modifier = Modifier.size(32.dp)) {
                    Icon(Icons.Default.Close, null, tint = BrainColors.text300, modifier = Modifier.size(20.dp))
                }
            }

            Spacer(Modifier.height(16.dp))

            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(10.dp))
                    .clickable {
                        onNewChat()
                        onDismiss()
                    },
                shape = RoundedCornerShape(10.dp),
                color = BrainColors.accentMain100.copy(alpha = 0.15f)
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Icon(Icons.Default.Add, null, tint = BrainColors.accentMain100, modifier = Modifier.size(20.dp))
                    Text("New chat", color = BrainColors.accentMain100, fontSize = 14.sp, fontWeight = FontWeight.Medium)
                }
            }

            Spacer(Modifier.height(16.dp))

            Text(
                text = "Recent",
                color = BrainColors.text400,
                fontSize = 11.sp,
                fontWeight = FontWeight.SemiBold,
                letterSpacing = 0.5.sp
            )

            Spacer(Modifier.height(8.dp))

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "No recent chats",
                    color = BrainColors.text500,
                    fontSize = 13.sp
                )
            }

            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(10.dp))
                    .clickable {
                        onSettings()
                        onDismiss()
                    },
                shape = RoundedCornerShape(10.dp),
                color = Color.Transparent
            ) {
                Row(
                    modifier = Modifier.padding(10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Icon(Icons.Default.Settings, null, tint = BrainColors.text300, modifier = Modifier.size(20.dp))
                    Text("Settings", color = BrainColors.text200, fontSize = 14.sp)
                }
            }
        }
    }
}
