package com.brain.app.features.message

import androidx.compose.animation.*
import androidx.compose.foundation.background
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.data.AgentEvent
import com.brain.app.theme.BrainColors
import com.brain.app.theme.BrainShapes

@Composable
fun MessageRenderer(
    events: List<AgentEvent>,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        events.forEach { event ->
            when (event) {
                is AgentEvent.Thought -> UserBubble(text = event.text)
                is AgentEvent.Text -> AssistantBubble(text = event.text)
                is AgentEvent.ToolCall -> ToolCallCard(event = event)
                is AgentEvent.ToolResult -> ToolResultBadge(event = event)
                is AgentEvent.Done -> DoneSummary(event = event)
                is AgentEvent.Error -> ErrorCard(event = event)
                is AgentEvent.FileRead -> FileReadBlock(event = event)
                is AgentEvent.TodoUpdate -> { /* skip for now */ }
                is AgentEvent.Init -> { /* skip */ }
            }
        }
    }
}

@Composable
fun UserBubble(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.End
    ) {
        Box(
            modifier = Modifier
                .widthIn(max = 300.dp)
                .clip(
                    RoundedCornerShape(
                        topStart = BrainShapes.messageOutgoing.topStart,
                        topEnd = BrainShapes.messageOutgoing.topEnd,
                        bottomStart = BrainShapes.messageOutgoing.bottomStart,
                        bottomEnd = BrainShapes.messageOutgoing.bottomEnd
                    )
                )
                .background(BrainColors.accentMain100)
                .padding(horizontal = 14.dp, vertical = 10.dp)
        ) {
            Text(
                text = text,
                color = Color.White,
                fontSize = 14.sp,
                lineHeight = 20.sp
            )
        }
    }
}

@Composable
fun AssistantBubble(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.Start
    ) {
        Box(
            modifier = Modifier
                .widthIn(max = 400.dp)
                .clip(
                    RoundedCornerShape(
                        topStart = BrainShapes.messageIncoming.topStart,
                        topEnd = BrainShapes.messageIncoming.topEnd,
                        bottomStart = BrainShapes.messageIncoming.bottomStart,
                        bottomEnd = BrainShapes.messageIncoming.bottomEnd
                    )
                )
                .background(BrainColors.bg300)
                .padding(horizontal = 14.dp, vertical = 10.dp)
        ) {
            Text(
                text = text,
                color = BrainColors.text100,
                fontSize = 14.sp,
                lineHeight = 20.sp
            )
        }
    }
}

@Composable
fun ToolCallCard(event: AgentEvent.ToolCall) {
    var expanded by remember { mutableStateOf(false) }

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .clickable { expanded = !expanded },
        shape = RoundedCornerShape(12.dp),
        color = BrainColors.bg300
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            // Icon box
            Box(
                modifier = Modifier
                    .size(32.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .background(BrainColors.accentMain100.copy(alpha = 0.15f)),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    toolIcon(event.tool),
                    contentDescription = null,
                    modifier = Modifier.size(16.dp),
                    tint = BrainColors.accentMain100
                )
            }

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = event.tool,
                    color = BrainColors.text100,
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = event.callId,
                    color = BrainColors.text400,
                    fontSize = 11.sp
                )
            }

            Icon(
                if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
                tint = BrainColors.text400
            )
        }
    }

    AnimatedVisibility(visible = expanded) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 44.dp, top = 4.dp)
                .clip(RoundedCornerShape(8.dp))
                .background(BrainColors.bg400)
                .padding(8.dp)
        ) {
            Text(
                text = event.args.toString(),
                color = BrainColors.text300,
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
                lineHeight = 16.sp
            )
        }
    }
}

@Composable
fun ToolResultBadge(event: AgentEvent.ToolResult) {
    Row(
        modifier = Modifier.padding(start = 44.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Icon(
            if (event.success) Icons.Default.CheckCircle else Icons.Default.Error,
            contentDescription = null,
            modifier = Modifier.size(14.dp),
            tint = if (event.success) BrainColors.success100 else BrainColors.danger100
        )
        Text(
            text = event.summary ?: (if (event.success) "ok" else "error"),
            color = BrainColors.text300,
            fontSize = 12.sp,
            maxLines = 1
        )
    }
}

@Composable
fun DoneSummary(event: AgentEvent.Done) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.Center
    ) {
        Surface(
            shape = BrainShapes.full,
            color = BrainColors.bg300
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                if (event.totalTokens > 0) {
                    StatChip("↑${formatTokens(event.totalTokens)}", BrainColors.text300)
                }
                if (event.totalCalls > 0) {
                    StatChip("⚡${event.totalCalls} tools", BrainColors.text300)
                }
                if (event.elapsedMs > 0) {
                    StatChip("⏱${String.format("%.1f", event.elapsedMs / 1000.0)}s", BrainColors.text300)
                }
            }
        }
    }
}

@Composable
fun ErrorCard(event: AgentEvent.Error) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        color = BrainColors.danger100.copy(alpha = 0.15f)
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                Icons.Default.Error,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
                tint = BrainColors.danger100
            )
            Text(
                text = event.message,
                color = BrainColors.danger100,
                fontSize = 13.sp
            )
        }
    }
}

@Composable
fun FileReadBlock(event: AgentEvent.FileRead) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(8.dp),
        color = BrainColors.bg300
    ) {
        Column(modifier = Modifier.padding(8.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp)
            ) {
                Icon(
                    Icons.Default.Description,
                    contentDescription = null,
                    modifier = Modifier.size(14.dp),
                    tint = BrainColors.accentMain100
                )
                Text(
                    text = event.path,
                    color = BrainColors.accentMain100,
                    fontSize = 11.sp,
                    fontFamily = FontFamily.Monospace
                )
            }
        }
    }
}

@Composable
private fun StatChip(text: String, color: Color) {
    Text(
        text = text,
        color = color,
        fontSize = 11.sp
    )
}

private fun toolIcon(tool: String) = when {
    tool.contains("file", ignoreCase = true) -> Icons.Default.Description
    tool.contains("shell", ignoreCase = true) || tool.contains("exec", ignoreCase = true) -> Icons.Default.Terminal
    tool.contains("todo", ignoreCase = true) -> Icons.Default.Checklist
    tool.contains("search", ignoreCase = true) -> Icons.Default.Search
    tool.contains("browser", ignoreCase = true) -> Icons.Default.Language
    tool.contains("list", ignoreCase = true) -> Icons.Default.Folder
    else -> Icons.Default.Build
}

private fun formatTokens(n: Int): String = when {
    n >= 10000 -> "${String.format("%.1f", n / 1000.0)}K"
    else -> "$n"
}
