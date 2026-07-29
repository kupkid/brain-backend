package com.brain.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import com.brain.app.ui.AgentChatScreen
import com.brain.app.ui.SettingsScreen
import com.brain.app.ui.theme.BrainTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val settings = BrainSettings(this)
        val repository = AgentRepository(settings)

        setContent {
            BrainTheme {
                var showSettings by remember { mutableStateOf(!settings.isConfigured) }
                val viewModel = remember { AgentViewModel(repository, settings) }

                AnimatedContent(
                    targetState = showSettings,
                    transitionSpec = {
                        if (targetState) slideInHorizontally { it } + fadeIn() togetherWith slideOutHorizontally { -it } + fadeOut()
                        else slideInHorizontally { -it } + fadeIn() togetherWith slideOutHorizontally { it } + fadeOut()
                    },
                    label = "screen"
                ) { settingsMode ->
                    if (settingsMode) {
                        SettingsScreen(
                            settings = settings,
                            onBack = { showSettings = false }
                        )
                    } else {
                        val events by viewModel.events.collectAsState()
                        val isRunning by viewModel.isRunning.collectAsState()
                        val selectedModel by settings.llmModel

                        AgentChatScreen(
                            events = events,
                            isRunning = isRunning,
                            selectedModel = selectedModel,
                            availableModels = settings.availableModels.value,
                            onModelSelected = { settings.saveModels(it, settings.embeddingModel.value) },
                            onSendTask = { task -> viewModel.sendTask(task) },
                            onStop = { viewModel.stopAgent() },
                            onSettings = { showSettings = true }
                        )
                    }
                }
            }
        }
    }
}
