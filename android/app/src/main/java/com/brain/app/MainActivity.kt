package com.brain.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import com.brain.app.ui.AgentChatScreen
import com.brain.app.ui.theme.BrainTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val host = intent.getStringExtra("agent_host") ?: "10.0.2.2:3000"
        val repository = AgentRepository(host)
        val viewModel = AgentViewModel(repository)

        setContent {
            BrainTheme {
                val events by viewModel.events.collectAsState()
                val isRunning by viewModel.isRunning.collectAsState()

                AgentChatScreen(
                    events = events,
                    isRunning = isRunning,
                    onSendTask = viewModel::sendTask,
                    onStop = viewModel::stopAgent
                )
            }
        }
    }
}
