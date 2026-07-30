package com.brain.app.features.chat

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.data.AgentEvent
import com.brain.app.features.message.MessageRenderer
import com.brain.app.theme.BrainColors
import kotlinx.coroutines.launch

@Composable
fun ChatArea(
    events: List<AgentEvent>,
    isRunning: Boolean,
    streamingText: String,
    modifier: Modifier = Modifier
) {
    val listState = rememberLazyListState()
    val coroutineScope = rememberCoroutineScope()

    // Auto-scroll to bottom on new events
    LaunchedEffect(events.size, streamingText) {
        if (events.isNotEmpty()) {
            listState.animateScrollToItem(events.size - 1)
        }
    }

    if (events.isEmpty() && !isRunning) {
        // Empty state handled by parent
        return
    }

    LazyColumn(
        state = listState,
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(bottom = 16.dp)
    ) {
        items(
            items = events,
            key = { "${it::class.simpleName}_${it.hashCode()}" }
        ) { event ->
            when (event) {
                is AgentEvent.Thought -> UserBubble(text = event.text)
                is AgentEvent.Text -> AssistantBubble(text = event.text)
                is AgentEvent.ToolCall -> com.brain.app.features.message.ToolCallCard(event = event)
                is AgentEvent.ToolResult -> com.brain.app.features.message.ToolResultBadge(event = event)
                is AgentEvent.Done -> com.brain.app.features.message.DoneSummary(event = event)
                is AgentEvent.Error -> com.brain.app.features.message.ErrorCard(event = event)
                is AgentEvent.FileRead -> com.brain.app.features.message.FileReadBlock(event = event)
                is AgentEvent.TodoUpdate -> {}
                is AgentEvent.Init -> {}
            }
        }

        // Streaming text indicator
        if (isRunning && streamingText.isBlank()) {
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(14.dp),
                        strokeWidth = 2.dp,
                        color = BrainColors.accentMain100
                    )
                    Text(
                        text = "Thinking...",
                        color = BrainColors.text400,
                        fontSize = 13.sp
                    )
                }
            }
        }

        // Streaming text
        if (streamingText.isNotBlank()) {
            item {
                AssistantBubble(text = streamingText)
            }
        }
    }
}
