package com.brain.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import com.brain.app.ui.AgentChatScreen
import com.brain.app.ui.SettingsScreen
import com.brain.app.ui.theme.BrainTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val settings = BrainSettings(this)

        setContent {
            BrainTheme {
                var screen by remember { mutableStateOf("chat") }

                val host = settings.serverHost.ifBlank { "10.0.2.2:3000" }
                val apiKey = settings.serverApiKey.ifBlank { null }
                val repository = remember(host, apiKey) { AgentRepository(host, apiKey) }
                val viewModel = remember { AgentViewModel(repository) }

                Scaffold(
                    bottomBar = {
                        NavigationBar(
                            containerColor = MaterialTheme.colorScheme.surfaceContainerLow
                        ) {
                            NavigationBarItem(
                                icon = { Icon(Icons.Default.Chat, null) },
                                label = { Text("Chat") },
                                selected = screen == "chat",
                                onClick = { screen = "chat" },
                                colors = NavigationBarItemDefaults.colors(
                                    selectedIconColor = MaterialTheme.colorScheme.primary,
                                    selectedTextColor = MaterialTheme.colorScheme.primary,
                                    unselectedIconColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                                    unselectedTextColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                                    indicatorColor = MaterialTheme.colorScheme.primaryContainer
                                )
                            )
                            NavigationBarItem(
                                icon = { Icon(Icons.Default.Settings, null) },
                                label = { Text("Settings") },
                                selected = screen == "settings",
                                onClick = { screen = "settings" },
                                colors = NavigationBarItemDefaults.colors(
                                    selectedIconColor = MaterialTheme.colorScheme.primary,
                                    selectedTextColor = MaterialTheme.colorScheme.primary,
                                    unselectedIconColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                                    unselectedTextColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                                    indicatorColor = MaterialTheme.colorScheme.primaryContainer
                                )
                            )
                        }
                    }
                ) { innerPadding ->
                    when (screen) {
                        "chat" -> {
                            val events by viewModel.events.collectAsState()
                            val isRunning by viewModel.isRunning.collectAsState()

                            Box(modifier = Modifier.padding(innerPadding)) {
                                AgentChatScreen(
                                    events = events,
                                    isRunning = isRunning,
                                    onSendTask = viewModel::sendTask,
                                    onStop = viewModel::stopAgent
                                )
                            }
                        }
                        "settings" -> {
                            Box(modifier = Modifier.padding(innerPadding)) {
                                SettingsScreen(
                                    settings = settings,
                                    onBack = { screen = "chat" }
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
