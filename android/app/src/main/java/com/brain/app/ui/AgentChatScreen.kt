package com.brain.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgentChatScreen(
    events: List<AgentEvent>,
    isRunning: Boolean,
    selectedModel: String,
    availableModels: List<ModelInfo>,
    onModelSelected: (String) -> Unit,
    onSendTask: (String) -> Unit,
    onStop: () -> Unit,
    onSettings: () -> Unit
) {
    var input by remember { mutableStateOf("") }
    var showModelMenu by remember { mutableStateOf(false) }
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
                        Text("Brain", fontWeight = FontWeight.Medium)
                        if (selectedModel.isNotBlank()) {
                            Text(
                                selectedModel,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                },
                navigationIcon = {
                    Box {
                        IconButton(onClick = { showModelMenu = true }) {
                            Icon(
                                Icons.Default.SmartToy,
                                contentDescription = "Model",
                                tint = if (selectedModel.isNotBlank()) MaterialTheme.colorScheme.primary
                                else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                                modifier = Modifier.size(22.dp)
                            )
                        }
                        DropdownMenu(
                            expanded = showModelMenu,
                            onDismissRequest = { showModelMenu = false }
                        ) {
                            if (availableModels.isEmpty()) {
                                DropdownMenuItem(
                                    text = { Text("No models loaded", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                                    onClick = { showModelMenu = false },
                                    leadingIcon = { Icon(Icons.Default.Info, null, modifier = Modifier.size(18.dp)) }
                                )
                            } else {
                                Text(
                                    "  Chat Model",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.padding(vertical = 4.dp)
                                )
                                availableModels.forEach { model ->
                                    DropdownMenuItem(
                                        text = {
                                            Text(
                                                model.id,
                                                color = if (model.id == selectedModel) MaterialTheme.colorScheme.primary
                                                else MaterialTheme.colorScheme.onSurface,
                                                fontWeight = if (model.id == selectedModel) FontWeight.Bold else FontWeight.Normal,
                                                fontSize = 14.sp,
                                                maxLines = 1,
                                                overflow = TextOverflow.Ellipsis
                                            )
                                        },
                                        onClick = {
                                            onModelSelected(model.id)
                                            showModelMenu = false
                                        },
                                        leadingIcon = {
                                            Icon(
                                                if (model.id == selectedModel) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                                                null,
                                                modifier = Modifier.size(18.dp),
                                                tint = if (model.id == selectedModel) MaterialTheme.colorScheme.primary
                                                else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                                            )
                                        }
                                    )
                                }
                            }
                        }
                    }
                },
                actions = {
                    if (isRunning) {
                        IconButton(onClick = onStop) {
                            Icon(Icons.Default.Close, "Stop", tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
                        }
                    }
                    IconButton(onClick = onSettings) {
                        Icon(Icons.Default.Settings, "Settings", modifier = Modifier.size(22.dp))
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                    titleContentColor = MaterialTheme.colorScheme.onSurface,
                )
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Messages
            if (events.isEmpty() && !isRunning) {
                Box(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            Icons.Default.SmartToy,
                            null,
                            modifier = Modifier.size(48.dp),
                            tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.1f)
                        )
                        Spacer(Modifier.height(12.dp))
                        Text(
                            "Enter a task",
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                            style = MaterialTheme.typography.bodyLarge
                        )
                        if (selectedModel.isBlank()) {
                            Spacer(Modifier.height(4.dp))
                            Text(
                                "Select a model first",
                                color = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f),
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    }
                }
            } else {
                LazyColumn(
                    state = listState,
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .padding(horizontal = 12.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                    contentPadding = PaddingValues(vertical = 8.dp)
                ) {
                    items(events, key = { "${it.type}_${it.ts}" }) { event ->
                        when (event) {
                            is ThoughtEvent -> ThoughtBubble(event)
                            is ToolCallEvent -> ToolCallCard(event)
                            is ToolResultEvent -> ToolResultBadge(event)
                            is TodoUpdateEvent -> TodoCard(event)
                            is FileReadEvent -> FileReadBlock(event)
                            is DoneEvent -> DoneCard(event)
                            is ErrorEvent -> ErrorCard(event)
                        }
                    }
                }
            }

            // Bottom input bar
            Surface(
                color = MaterialTheme.colorScheme.surfaceContainerLow,
                shadowElevation = 8.dp,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    OutlinedTextField(
                        value = input,
                        onValueChange = { input = it },
                        modifier = Modifier
                            .weight(1f)
                            .focusRequester(focusRequester),
                        placeholder = {
                            Text(
                                "Ask anything...",
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.35f)
                            )
                        },
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
                        keyboardActions = KeyboardActions(
                            onSend = {
                                if (input.isNotBlank() && !isRunning) {
                                    onSendTask(input)
                                    input = ""
                                    focusManager.clearFocus()
                                }
                            }
                        ),
                        singleLine = false,
                        maxLines = 5,
                    )

                    FilledIconButton(
                        onClick = {
                            if (input.isNotBlank() && !isRunning) {
                                onSendTask(input)
                                input = ""
                                focusManager.clearFocus()
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
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                            )
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
fun ThoughtBubble(event: ThoughtEvent) {
    Text(
        text = event.text,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        style = MaterialTheme.typography.bodyMedium,
        modifier = Modifier.padding(start = 4.dp, top = 4.dp)
    )
}

@Composable
fun ToolCallCard(event: ToolCallEvent) {
    val argsPreview = buildString {
        val entries = event.args.entries.take(2)
        entries.forEachIndexed { i, (k, v) ->
            if (i > 0) append(", ")
            val value = when (v) {
                is kotlinx.serialization.json.JsonPrimitive -> v.content
                else -> v.toString().take(40)
            }
            append("$k=${value.take(50)}")
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                Icons.Default.Code,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(16.dp)
            )
            Spacer(Modifier.width(8.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = event.tool,
                    color = MaterialTheme.colorScheme.primary,
                    fontWeight = FontWeight.Medium,
                    fontSize = 13.sp
                )
                if (argsPreview.isNotEmpty()) {
                    Text(
                        text = argsPreview,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 12.sp,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                }
            }
            Text(
                text = event.call_id,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace
            )
        }
    }
}

@Composable
fun ToolResultBadge(event: ToolResultEvent) {
    Row(
        modifier = Modifier.padding(start = 28.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            if (event.success) Icons.Default.CheckCircle else Icons.Default.Error,
            contentDescription = null,
            tint = if (event.success) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
            modifier = Modifier.size(14.dp)
        )
        Spacer(Modifier.width(6.dp))
        Text(
            text = event.summary.take(120),
            color = if (event.success) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error,
            fontSize = 12.sp,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis
        )
    }
}

@Composable
fun TodoCard(event: TodoUpdateEvent) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(10.dp)) {
            val done = event.todos.count { it.status == "done" }
            val total = event.todos.size
            if (total > 0) {
                LinearProgressIndicator(
                    progress = { done.toFloat() / total },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(3.dp)
                        .clip(RoundedCornerShape(2.dp)),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.surfaceVariant,
                )
                Spacer(Modifier.height(6.dp))
            }
            event.todos.forEach { todo ->
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(vertical = 2.dp)
                ) {
                    Icon(
                        if (todo.status == "done") Icons.Default.Check else Icons.Default.Send,
                        contentDescription = null,
                        tint = if (todo.status == "done") MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(14.dp)
                    )
                    Spacer(Modifier.width(6.dp))
                    Text(
                        text = todo.text,
                        color = if (todo.status == "done") MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
                        fontSize = 13.sp
                    )
                }
            }
        }
    }
}

@Composable
fun FileReadBlock(event: FileReadEvent) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(10.dp)) {
            Text(
                text = event.path,
                color = MaterialTheme.colorScheme.primary,
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Medium
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = event.text.take(500),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
                maxLines = 15,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
fun DoneCard(event: DoneEvent) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.15f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = event.summary,
                color = MaterialTheme.colorScheme.primary,
                fontWeight = FontWeight.Medium,
                fontSize = 14.sp
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = "${event.total_tokens} tokens · ${event.total_calls} tools",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                fontSize = 12.sp
            )
        }
    }
}

@Composable
fun ErrorCard(event: ErrorEvent) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.15f)),
        shape = RoundedCornerShape(10.dp)
    ) {
        Text(
            text = event.message,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(12.dp),
            fontSize = 14.sp
        )
    }
}
