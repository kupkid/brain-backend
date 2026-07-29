package com.brain.app.ui

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.*
import kotlinx.serialization.json.JsonPrimitive

// ─── Shimmer ────────────────────────────────────────────────────────────
@Composable
fun shimmerBrush(): Brush {
    val shimmerColors = listOf(
        Color(0xFF2A2A2A), Color(0xFF1A1A1A), Color(0xFF2A2A2A)
    )
    val transition = rememberInfiniteTransition(label = "shimmer")
    val translateAnim = transition.animateFloat(
        initialValue = 0f, targetValue = 1000f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Restart
        ), label = "shimmer"
    )
    return Brush.linearGradient(
        colors = shimmerColors,
        start = Offset.Zero,
        end = Offset(x = translateAnim.value, y = translateAnim.value)
    )
}

@Composable
fun toolIcon(toolName: String) = when {
    toolName.contains("grep", true) || toolName.contains("search", true) -> Icons.Default.Search
    toolName.contains("file", true) || toolName.contains("read", true) || toolName.contains("write", true) -> Icons.Default.Description
    toolName.contains("shell", true) || toolName.contains("exec", true) -> Icons.Default.Terminal
    toolName.contains("todo", true) -> Icons.Default.Checklist
    toolName.contains("browser", true) || toolName.contains("navigate", true) -> Icons.Default.Language
    toolName.contains("list", true) || toolName.contains("dir", true) -> Icons.Default.Folder
    else -> Icons.Default.Build
}

// ─── Main Chat Screen ───────────────────────────────────────────────────
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgentChatScreen(
    events: List<AgentEvent>,
    isRunning: Boolean,
    selectedModel: String,
    chatTitle: String,
    availableModels: List<String> = emptyList(),
    onSendTask: (String) -> Unit,
    onStop: () -> Unit,
    onMenuClick: () -> Unit,
    onNewChat: () -> Unit,
    onSettings: () -> Unit,
    onModelSelected: (String) -> Unit = {}
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
                                selectedModel.split("/").last(),
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f),
                                maxLines = 1, overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onMenuClick) { Icon(Icons.Default.Menu, "Menu") }
                },
                actions = {
                    IconButton(onClick = onNewChat) { Icon(Icons.Default.Add, "New chat") }
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
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            // Messages area
            if (events.isEmpty() && !isRunning) {
                // ─── Empty state ───
                Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier.padding(horizontal = 32.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(72.dp)
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.08f)),
                            contentAlignment = Alignment.Center
                        ) {
                            Icon(Icons.Default.AutoAwesome, null,
                                modifier = Modifier.size(36.dp),
                                tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.3f))
                        }
                        Spacer(Modifier.height(20.dp))
                        Text("Чем могу помочь?",
                            fontSize = 22.sp, fontWeight = FontWeight.SemiBold,
                            color = MaterialTheme.colorScheme.onSurface)
                        Spacer(Modifier.height(8.dp))
                        Text("Задайте вопрос или поручите задачу",
                            fontSize = 14.sp,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f))
                    }
                }
            } else {
                LazyColumn(
                    state = listState,
                    modifier = Modifier.weight(1f).fillMaxWidth().padding(horizontal = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
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
                            is ToolCallEvent -> SkeletonToolCall(event, events, idx)
                            is ToolResultEvent -> ToolResultInline(event)
                            is TodoUpdateEvent -> TodoProgress(event)
                            is FileReadEvent -> FileReadBlock(event)
                            is TextEvent -> StreamingText(event)
                            is DoneEvent -> ResponseBlock(event)
                            is ErrorEvent -> ErrorBlock(event)
                        }
                    }
                }
            }

            // ─── Input area ───
            ChatInputArea(
                input = input,
                onInputChange = { input = it },
                onSend = {
                    if (input.isNotBlank() && !isRunning) {
                        onSendTask(input); input = ""; focusManager.clearFocus()
                    }
                },
                isRunning = isRunning,
                selectedModel = selectedModel,
                availableModels = availableModels,
                onModelSelected = onModelSelected,
                focusRequester = focusRequester,
            )
        }
    }
}

// ─── User Bubble ─────────────────────────────────────────────────────────
@Composable
fun UserBubble(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(start = 48.dp, end = 0.dp, top = 8.dp),
        horizontalArrangement = Arrangement.End
    ) {
        Surface(
            shape = RoundedCornerShape(20.dp, 20.dp, 6.dp, 20.dp),
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.widthIn(max = 320.dp)
        ) {
            Text(
                text = text, color = MaterialTheme.colorScheme.onPrimary,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                fontSize = 15.sp, lineHeight = 22.sp
            )
        }
    }
}

// ─── Skeleton Tool Call ──────────────────────────────────────────────────
@Composable
fun SkeletonToolCall(event: ToolCallEvent, allEvents: List<AgentEvent>, index: Int) {
    var expanded by remember { mutableStateOf(false) }
    val hasResult = allEvents.drop(index + 1).takeWhile {
        it is ToolCallEvent || it is ToolResultEvent
    }.filterIsInstance<ToolResultEvent>().any { it.call_id == event.call_id }

    val icon = toolIcon(event.tool)
    val argsPreview = buildString {
        event.args.entries.take(1).forEach { (_, v) ->
            val value = when (v) {
                is JsonPrimitive -> v.content
                else -> v.toString().take(40)
            }
            append(value.take(60))
        }
    }

    Column(modifier = Modifier.padding(start = 0.dp, end = 48.dp, top = 6.dp)) {
        Surface(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(12.dp))
                .clickable { if (hasResult) expanded = !expanded },
            color = if (hasResult) MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.5f)
            else Color.Transparent,
            shape = RoundedCornerShape(12.dp),
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Box(
                    modifier = Modifier
                        .size(28.dp)
                        .clip(RoundedCornerShape(8.dp))
                        .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.1f)),
                    contentAlignment = Alignment.Center
                ) {
                    Icon(icon, null, tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.7f), modifier = Modifier.size(16.dp))
                }
                if (hasResult) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            event.tool,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f),
                            fontSize = 13.sp, fontWeight = FontWeight.Medium
                        )
                        if (argsPreview.isNotBlank()) {
                            Text(
                                argsPreview.take(80),
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f),
                                fontSize = 12.sp, fontFamily = FontFamily.Monospace,
                                maxLines = 1, overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                    Icon(
                        if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        null, modifier = Modifier.size(16.dp),
                        tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                    )
                } else {
                    Box(modifier = Modifier.weight(1f).height(16.dp).clip(RoundedCornerShape(4.dp))) {
                        Box(modifier = Modifier.fillMaxSize().background(brush = shimmerBrush()))
                    }
                    CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 1.5.dp,
                        color = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f))
                }
            }
        }

        // Expanded details
        if (expanded && hasResult) {
            val resultEvent = allEvents.drop(index + 1).takeWhile {
                it is ToolCallEvent || it is ToolResultEvent
            }.filterIsInstance<ToolResultEvent>().firstOrNull { it.call_id == event.call_id }
            if (resultEvent != null && resultEvent.summary.isNotBlank()) {
                Surface(
                    modifier = Modifier.padding(start = 12.dp, top = 2.dp).fillMaxWidth(),
                    color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.3f),
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Text(
                        resultEvent.summary,
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                        fontSize = 12.sp,
                        fontFamily = FontFamily.Monospace,
                        maxLines = 12,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(10.dp)
                    )
                }
            }
        }
    }
}

// ─── Tool Result (inline) ───────────────────────────────────────────────
@Composable
fun ToolResultInline(event: ToolResultEvent) {
    Row(
        modifier = Modifier.padding(start = 52.dp, end = 48.dp, top = 2.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            if (event.success) Icons.Default.CheckCircle else Icons.Default.Error,
            null,
            tint = if (event.success) Color(0xFF4CAF50).copy(alpha = 0.7f) else MaterialTheme.colorScheme.error.copy(alpha = 0.7f),
            modifier = Modifier.size(12.dp)
        )
        Spacer(Modifier.width(6.dp))
        Text(
            event.summary.take(120),
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f),
            fontSize = 12.sp,
            maxLines = 1, overflow = TextOverflow.Ellipsis
        )
    }
}

// ─── Todo Progress ──────────────────────────────────────────────────────
@Composable
fun TodoProgress(event: TodoUpdateEvent) {
    val done = event.todos.count { it.status == "done" }
    Column(modifier = Modifier.fillMaxWidth().padding(end = 48.dp, top = 4.dp)) {
        if (event.todos.isNotEmpty()) {
            LinearProgressIndicator(
                progress = { done.toFloat() / event.todos.size },
                modifier = Modifier.fillMaxWidth().height(2.dp).clip(RoundedCornerShape(2.dp)),
                color = MaterialTheme.colorScheme.primary,
                trackColor = MaterialTheme.colorScheme.surfaceVariant,
            )
        }
        Spacer(Modifier.height(4.dp))
        event.todos.forEach { todo ->
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 1.dp)) {
                Icon(
                    if (todo.status == "done") Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                    null,
                    tint = if (todo.status == "done") Color(0xFF4CAF50) else MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(14.dp)
                )
                Spacer(Modifier.width(6.dp))
                Text(todo.text,
                    color = if (todo.status == "done") MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f) else MaterialTheme.colorScheme.onSurface,
                    fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

// ─── File Read Block ────────────────────────────────────────────────────
@Composable
fun FileReadBlock(event: FileReadEvent) {
    Column(modifier = Modifier.fillMaxWidth().padding(end = 48.dp, top = 4.dp)) {
        Surface(
            shape = RoundedCornerShape(10.dp),
            color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.4f)
        ) {
            Column(Modifier.padding(10.dp)) {
                Text(event.path, color = MaterialTheme.colorScheme.primary.copy(alpha = 0.8f),
                    fontSize = 11.sp, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium)
                Spacer(Modifier.height(4.dp))
                Text(event.text.take(400), color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                    fontSize = 11.sp, fontFamily = FontFamily.Monospace, maxLines = 12, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

// ─── Streaming Text ─────────────────────────────────────────────────────
@Composable
fun StreamingText(event: TextEvent) {
    val clipboardManager = LocalClipboardManager.current
    Column(
        modifier = Modifier.fillMaxWidth().padding(top = 12.dp, end = 8.dp)
    ) {
        Text(
            text = event.text,
            color = MaterialTheme.colorScheme.onSurface,
            fontSize = 15.sp,
            lineHeight = 23.sp
        )
        Row(modifier = Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            IconButton(onClick = { clipboardManager.setText(AnnotatedString(event.text)) }, modifier = Modifier.size(28.dp)) {
                Icon(Icons.Default.ContentCopy, "Copy", modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
            }
        }
    }
}

// ─── Response Block + Stats ─────────────────────────────────────────────
@Composable
fun ResponseBlock(event: DoneEvent) {
    val clipboardManager = LocalClipboardManager.current
    Column(modifier = Modifier.fillMaxWidth().padding(top = 4.dp, end = 8.dp)) {
        if (event.summary.isNotBlank()) {
            Text(
                text = event.summary,
                color = MaterialTheme.colorScheme.onSurface,
                fontSize = 15.sp,
                lineHeight = 23.sp
            )
        }

        // Action row
        Row(modifier = Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
            AssistChip(
                onClick = { clipboardManager.setText(AnnotatedString(event.summary)) },
                label = { Text("Копировать", fontSize = 11.sp) },
                leadingIcon = { Icon(Icons.Default.ContentCopy, null, Modifier.size(14.dp)) },
                modifier = Modifier.height(28.dp),
                shape = RoundedCornerShape(8.dp),
                colors = AssistChipDefaults.assistChipColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
            )
            AssistChip(
                onClick = { /* TODO: regenerate */ },
                label = { Text("Ещё раз", fontSize = 11.sp) },
                leadingIcon = { Icon(Icons.Default.Refresh, null, Modifier.size(14.dp)) },
                modifier = Modifier.height(28.dp),
                shape = RoundedCornerShape(8.dp),
                colors = AssistChipDefaults.assistChipColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
            )
        }

        // Stats bar
        if (event.total_tokens > 0) {
            Row(
                modifier = Modifier.padding(top = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                val fmt = { n: Int ->
                    when {
                        n >= 1000 -> "${"%.1f".format(n / 1000f)}K"
                        else -> "$n"
                    }
                }
                val fmtD = { d: Double ->
                    when {
                        d >= 1000 -> "${"%.1f".format(d / 1000f)}K"
                        else -> "${"%.1f".format(d)}"
                    }
                }
                Text(
                    "↑${fmt(event.tokens_input)} ↓${fmt(event.tokens_output)}",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f),
                    fontSize = 11.sp
                )
                Text(
                    "⚡${fmtD(event.tokens_per_sec)} tok/s",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f),
                    fontSize = 11.sp
                )
                val seconds = event.elapsed_ms / 1000.0
                Text(
                    "⏱${"%.1f".format(seconds)}s",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f),
                    fontSize = 11.sp
                )
            }
        }
    }
}

// ─── Error Block ────────────────────────────────────────────────────────
@Composable
fun ErrorBlock(event: ErrorEvent) {
    Surface(
        modifier = Modifier.fillMaxWidth().padding(end = 48.dp, top = 4.dp),
        color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.15f),
        shape = RoundedCornerShape(10.dp)
    ) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.Default.Warning, null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(16.dp))
            Spacer(Modifier.width(8.dp))
            Text(event.message, color = MaterialTheme.colorScheme.error, fontSize = 13.sp)
        }
    }
}

// ─── Chat Input Area — full rounded rectangle with model selector ───────
@Composable
fun ChatInputArea(
    input: String,
    onInputChange: (String) -> Unit,
    onSend: () -> Unit,
    isRunning: Boolean,
    selectedModel: String,
    availableModels: List<String>,
    onModelSelected: (String) -> Unit,
    focusRequester: FocusRequester,
) {
    var showModelPicker by remember { mutableStateOf(false) }
    val shortModel = selectedModel.split("/").last().take(20)

    Surface(
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        shadowElevation = 8.dp,
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp)) {
            // Input field — large rounded rectangle
            Surface(
                shape = RoundedCornerShape(20.dp),
                color = MaterialTheme.colorScheme.surfaceContainer,
                modifier = Modifier.fillMaxWidth()
            ) {
                Column {
                    OutlinedTextField(
                        value = input,
                        onValueChange = onInputChange,
                        modifier = Modifier
                            .fillMaxWidth()
                            .focusRequester(focusRequester),
                        placeholder = {
                            Text("Сообщение...",
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                                fontSize = 15.sp)
                        },
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = Color.Transparent,
                            unfocusedBorderColor = Color.Transparent,
                            cursorColor = MaterialTheme.colorScheme.primary,
                            focusedTextColor = MaterialTheme.colorScheme.onSurface,
                            unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
                            focusedContainerColor = Color.Transparent,
                            unfocusedContainerColor = Color.Transparent,
                        ),
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                        keyboardActions = KeyboardActions(onSend = { onSend() }),
                        maxLines = 6,
                    )

                    // Bottom row: model chip + send
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 12.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        // Model selector chip
                        if (shortModel.isNotBlank()) {
                            Surface(
                                modifier = Modifier.clickable { showModelPicker = !showModelPicker },
                                shape = RoundedCornerShape(12.dp),
                                color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.6f),
                            ) {
                                Row(
                                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(4.dp)
                                ) {
                                    Icon(Icons.Default.SmartToy, null, Modifier.size(14.dp),
                                        tint = MaterialTheme.colorScheme.primary.copy(alpha = 0.7f))
                                    Text(shortModel, fontSize = 12.sp,
                                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                                        maxLines = 1, overflow = TextOverflow.Ellipsis)
                                    Icon(Icons.Default.ExpandMore, null, Modifier.size(12.dp),
                                        tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
                                }
                            }
                        } else {
                            Spacer(Modifier.height(1.dp))
                        }

                        // Send button
                        FilledIconButton(
                            onClick = onSend,
                            enabled = input.isNotBlank() && !isRunning,
                            modifier = Modifier.size(40.dp),
                            shape = RoundedCornerShape(12.dp),
                            colors = IconButtonDefaults.filledIconButtonColors(
                                containerColor = MaterialTheme.colorScheme.primary,
                                contentColor = MaterialTheme.colorScheme.onPrimary,
                                disabledContainerColor = MaterialTheme.colorScheme.surfaceVariant,
                                disabledContentColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.2f)
                            )
                        ) {
                            if (isRunning) {
                                CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp,
                                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
                            } else {
                                Icon(Icons.Default.ArrowUpward, "Send", modifier = Modifier.size(20.dp))
                            }
                        }
                    }
                }
            }

            // Model picker dropdown
            AnimatedVisibility(visible = showModelPicker && availableModels.size > 1) {
                Surface(
                    modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
                    shape = RoundedCornerShape(14.dp),
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                    shadowElevation = 4.dp,
                ) {
                    Column(modifier = Modifier.padding(8.dp)) {
                        Text("Модель", fontSize = 12.sp, fontWeight = FontWeight.Medium,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp))
                        availableModels.take(8).forEach { model ->
                            val isSelected = model == selectedModel
                            val displayName = model.split("/").last()
                            Surface(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(10.dp))
                                    .clickable { onModelSelected(model); showModelPicker = false },
                                color = if (isSelected) MaterialTheme.colorScheme.primary.copy(alpha = 0.15f) else Color.Transparent
                            ) {
                                Row(
                                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    if (isSelected) {
                                        Icon(Icons.Default.CheckCircle, null, Modifier.size(16.dp),
                                            tint = MaterialTheme.colorScheme.primary)
                                        Spacer(Modifier.width(8.dp))
                                    }
                                    Text(
                                        displayName,
                                        fontSize = 14.sp,
                                        color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                                        modifier = Modifier.weight(1f),
                                        maxLines = 1, overflow = TextOverflow.Ellipsis
                                    )
                                }
                            }
                        }
                        if (availableModels.size > 8) {
                            Text("+${availableModels.size - 8} ещё", fontSize = 12.sp,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                                modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp))
                        }
                    }
                }
            }
        }
    }
}
