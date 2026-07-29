package com.brain.app.ui

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.*
import kotlinx.serialization.json.JsonPrimitive

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgentChatScreen(
    events: List<AgentEvent>,
    isRunning: Boolean,
    selectedModel: String,
    chatTitle: String,
    onSendTask: (String) -> Unit,
    onStop: () -> Unit,
    onMenuClick: () -> Unit,
    onNewChat: () -> Unit,
    onSettings: () -> Unit
) {
    var input by remember { mutableStateOf("") }
    val listState = rememberLazyListState()
    val focusRequester = remember { FocusRequester() }
    val focusManager = LocalFocusManager.current

    LaunchedEffect(events.size) {
        if (events.isNotEmpty()) {
            listState.animateScrollToItem(events.size - 1)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(chatTitle, fontWeight = FontWeight.Medium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        if (selectedModel.isNotBlank()) {
                            Text(
                                selectedModel,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onMenuClick) {
                        Icon(Icons.Default.Menu, "Menu")
                    }
                },
                actions = {
                    IconButton(onClick = onNewChat) {
                        Icon(Icons.Default.Add, "New chat")
                    }
                    if (isRunning) {
                        IconButton(onClick = onStop) {
                            Icon(Icons.Default.Close, "Stop", tint = MaterialTheme.colorScheme.error)
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding)
        ) {
            // Messages
            if (events.isEmpty() && !isRunning) {
                Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(Icons.Default.SmartToy, null, modifier = Modifier.size(48.dp), tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f))
                        Spacer(Modifier.height(12.dp))
                        Text("Ask anything...", color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f), style = MaterialTheme.typography.bodyLarge)
                    }
                }
            } else {
                LazyColumn(
                    state = listState,
                    modifier = Modifier.weight(1f).fillMaxWidth().padding(horizontal = 12.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                    contentPadding = PaddingValues(vertical = 8.dp)
                ) {
                    items(events.size) { idx ->
                        val event = events[idx]
                        when (event) {
                            is ThoughtEvent -> {
                                if (idx == 0 || events.getOrNull(idx - 1) !is ThoughtEvent) {
                                    UserBubble(event.text)
                                }
                            }
                            is ToolCallEvent -> AnimatedToolCall(event)
                            is ToolResultEvent -> ToolResultBadge(event)
                            is TodoUpdateEvent -> TodoCard(event)
                            is FileReadEvent -> FileReadBlock(event)
                            is DoneEvent -> DoneCard(event)
                            is ErrorEvent -> ErrorCard(event)
                        }
                    }
                }
            }

            // Input bar
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainerLow,
                shadowElevation = 8.dp,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.Bottom,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    OutlinedTextField(
                        value = input,
                        onValueChange = { input = it },
                        modifier = Modifier.weight(1f).focusRequester(focusRequester),
                        placeholder = { Text("Ask anything...", color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.35f)) },
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f),
                            unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f),
                            cursorColor = MaterialTheme.colorScheme.primary,
                            focusedTextColor = MaterialTheme.colorScheme.onSurface,
                            unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
                            focusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                            unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainer,
                        ),
                        shape = RoundedCornerShape(24.dp),
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                        keyboardActions = KeyboardActions(onSend = {
                            if (input.isNotBlank() && !isRunning) {
                                onSendTask(input); input = ""; focusManager.clearFocus()
                            }
                        }),
                        maxLines = 5,
                    )
                    FilledIconButton(
                        onClick = {
                            if (input.isNotBlank() && !isRunning) {
                                onSendTask(input); input = ""; focusManager.clearFocus()
                            }
                        },
                        enabled = input.isNotBlank() && !isRunning,
                        modifier = Modifier.size(48.dp),
                        shape = RoundedCornerShape(16.dp),
                        colors = IconButtonDefaults.filledIconButtonColors(
                            containerColor = MaterialTheme.colorScheme.primary,
                            contentColor = MaterialTheme.colorScheme.onPrimary,
                            disabledContainerColor = MaterialTheme.colorScheme.surfaceVariant,
                            disabledContentColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.2f)
                        )
                    ) {
                        if (isRunning) {
                            CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp, color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
                        } else {
                            Icon(Icons.Default.ArrowUpward, "Send", modifier = Modifier.size(22.dp))
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun UserBubble(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(start = 48.dp, end = 0.dp, top = 4.dp),
        horizontalArrangement = Arrangement.End
    ) {
        Surface(
            shape = RoundedCornerShape(18.dp, 18.dp, 4.dp, 18.dp),
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.widthIn(max = 320.dp)
        ) {
            Text(
                text = text,
                color = MaterialTheme.colorScheme.onPrimary,
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                fontSize = 15.sp
            )
        }
    }
}

@Composable
fun AnimatedToolCall(event: ToolCallEvent) {
    var expanded by remember { mutableStateOf(false) }
    val argsPreview = buildString {
        event.args.entries.take(2).forEachIndexed { i, (k, v) ->
            if (i > 0) append(" ")
            val value = when (v) {
                is JsonPrimitive -> v.content
                else -> v.toString().take(40)
            }
            append("$k=${value.take(60)}")
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth().padding(start = 0.dp, end = 48.dp, top = 2.dp),
        onClick = { expanded = !expanded },
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Default.Code, null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(14.dp))
                Spacer(Modifier.width(6.dp))
                Text(event.tool, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Medium, fontSize = 13.sp)
                Spacer(Modifier.weight(1f))
                Icon(
                    if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    null, modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                )
            }
            if (argsPreview.isNotEmpty()) {
                Text(argsPreview, color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp, maxLines = if (expanded) 10 else 1, overflow = TextOverflow.Ellipsis, fontFamily = FontFamily.Monospace)
            }
            AnimatedVisibility(visible = expanded) {
                Text(
                    event.args.toString().take(500),
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                    fontSize = 11.sp,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(top = 4.dp)
                )
            }
        }
    }
}

@Composable
fun ToolResultBadge(event: ToolResultEvent) {
    Row(
        modifier = Modifier.padding(start = 8.dp, end = 48.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            if (event.success) Icons.Default.CheckCircle else Icons.Default.Error,
            null,
            tint = if (event.success) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
            modifier = Modifier.size(14.dp)
        )
        Spacer(Modifier.width(6.dp))
        Text(
            event.summary.take(150),
            color = if (event.success) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error,
            fontSize = 12.sp,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis
        )
    }
}

@Composable
fun TodoCard(event: TodoUpdateEvent) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(end = 48.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(10.dp)) {
            val done = event.todos.count { it.status == "done" }
            if (event.todos.isNotEmpty()) {
                LinearProgressIndicator(
                    progress = { done.toFloat() / event.todos.size },
                    modifier = Modifier.fillMaxWidth().height(3.dp).clip(RoundedCornerShape(2.dp)),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.surfaceVariant,
                )
                Spacer(Modifier.height(6.dp))
            }
            event.todos.forEach { todo ->
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 2.dp)) {
                    Icon(
                        if (todo.status == "done") Icons.Default.Check else Icons.Default.Send,
                        null,
                        tint = if (todo.status == "done") MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(14.dp)
                    )
                    Spacer(Modifier.width(6.dp))
                    Text(todo.text, color = if (todo.status == "done") MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface, fontSize = 13.sp)
                }
            }
        }
    }
}

@Composable
fun FileReadBlock(event: FileReadEvent) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(end = 48.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(10.dp)) {
            Text(event.path, color = MaterialTheme.colorScheme.primary, fontSize = 12.sp, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium)
            Spacer(Modifier.height(4.dp))
            Text(event.text.take(500), color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp, fontFamily = FontFamily.Monospace, maxLines = 15, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
fun DoneCard(event: DoneEvent) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(end = 48.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.15f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(event.summary, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Medium, fontSize = 14.sp)
            Spacer(Modifier.height(4.dp))
            Text("${event.total_tokens} tokens · ${event.total_calls} tools", color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 12.sp)
        }
    }
}

@Composable
fun ErrorCard(event: ErrorEvent) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(end = 48.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.15f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Text(event.message, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(12.dp), fontSize = 14.sp)
    }
}
