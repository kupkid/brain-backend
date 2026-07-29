package com.brain.app.ui

import androidx.compose.animation.*
import androidx.compose.animation.core.*
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

// ─── Skeleton shimmer animation ─────────────────────────────────────────
@Composable
fun shimmerBrush(): Brush {
    val shimmerColors = listOf(
        Color.Gray.copy(alpha = 0.25f),
        Color.Gray.copy(alpha = 0.10f),
        Color.Gray.copy(alpha = 0.25f),
    )
    val transition = rememberInfiniteTransition(label = "shimmer")
    val translateAnim = transition.animateFloat(
        initialValue = 0f, targetValue = 1000f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Restart
        ), label = "shimmer_translate"
    )
    return Brush.linearGradient(
        colors = shimmerColors,
        start = Offset.Zero,
        end = Offset(x = translateAnim.value, y = translateAnim.value)
    )
}

// ─── Tool icon helper ───────────────────────────────────────────────────
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

    // Auto-scroll on new events
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
                Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(Icons.Default.SmartToy, null, modifier = Modifier.size(48.dp),
                            tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.08f))
                        Spacer(Modifier.height(12.dp))
                        Text("Ask anything...", color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                            style = MaterialTheme.typography.bodyLarge)
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
                            // ─── User message (right bubble) ───
                            is ThoughtEvent -> {
                                if (idx == 0 || events.getOrNull(idx - 1) !is ThoughtEvent) {
                                    UserBubble(event.text)
                                }
                            }
                            // ─── Skeleton tool call (no border, shimmer) ───
                            is ToolCallEvent -> SkeletonToolCall(event, events, idx)
                            // ─── Tool result (inline under tool) ───
                            is ToolResultEvent -> ToolResultInline(event)
                            // ─── Todo progress ───
                            is TodoUpdateEvent -> TodoProgress(event)
                            // ─── File read content ───
                            is FileReadEvent -> FileReadBlock(event)
                            // ─── Streaming text (no bubble, full width) ───
                            is TextEvent -> StreamingText(event)
                            // ─── Final response + stats ───
                            is DoneEvent -> ResponseBlock(event)
                            // ─── Error ───
                            is ErrorEvent -> ErrorBlock(event)
                        }
                    }
                }
            }

            // ─── Model selector chips (if multiple models) ───
            if (availableModels.size > 1) {
                ModelChipsBar(
                    models = availableModels,
                    selected = selectedModel,
                    onSelect = onModelSelected
                )
            }

            // ─── Input bar ───
            InputBar(
                input = input,
                onInputChange = { input = it },
                onSend = {
                    if (input.isNotBlank() && !isRunning) {
                        onSendTask(input); input = ""; focusManager.clearFocus()
                    }
                },
                isRunning = isRunning,
                focusRequester = focusRequester
            )
        }
    }
}

// ─── User Bubble ─────────────────────────────────────────────────────────
@Composable
fun UserBubble(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(start = 48.dp, end = 0.dp, top = 6.dp),
        horizontalArrangement = Arrangement.End
    ) {
        Surface(
            shape = RoundedCornerShape(18.dp, 18.dp, 4.dp, 18.dp),
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.widthIn(max = 320.dp)
        ) {
            Text(
                text = text, color = MaterialTheme.colorScheme.onPrimary,
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                fontSize = 15.sp
            )
        }
    }
}

// ─── Skeleton Tool Call (shimmer, no border) ────────────────────────────
@Composable
fun SkeletonToolCall(event: ToolCallEvent, allEvents: List<AgentEvent>, index: Int) {
    var expanded by remember { mutableStateOf(false) }
    // Check if result already arrived
    val hasResult = allEvents.drop(index + 1).takeWhile {
        it is ToolCallEvent || it is ToolResultEvent
    }.filterIsInstance<ToolResultEvent>().any { it.call_id == event.call_id }

    val icon = toolIcon(event.tool)
    val argsPreview = buildString {
        event.args.entries.take(1).forEach { (k, v) ->
            val value = when (v) {
                is JsonPrimitive -> v.content
                else -> v.toString().take(40)
            }
            append("${value.take(60)}")
        }
    }

    Column(modifier = Modifier.padding(start = 0.dp, end = 48.dp, top = 4.dp)) {
        // Tool header row
        Row(
            modifier = Modifier.clickable { expanded = !expanded },
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Icon(icon, null, tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f), modifier = Modifier.size(16.dp))
            if (hasResult) {
                // Done — show tool name + preview
                Text(
                    "${event.tool} $argsPreview",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                    fontSize = 13.sp,
                    maxLines = 1, overflow = TextOverflow.Ellipsis
                )
                Icon(
                    if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    null, modifier = Modifier.size(14.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f)
                )
            } else {
                // Running — shimmer skeleton text
                Box(modifier = Modifier.height(16.dp).fillMaxWidth(0.6f).clip(RoundedCornerShape(4.dp))) {
                    Box(modifier = Modifier.fillMaxSize().background(brush = shimmerBrush()))
                }
            }
        }

        // Expanded details
        if (expanded && hasResult) {
            val resultEvent = allEvents.drop(index + 1).takeWhile {
                it is ToolCallEvent || it is ToolResultEvent
            }.filterIsInstance<ToolResultEvent>().firstOrNull { it.call_id == event.call_id }
            if (resultEvent != null && resultEvent.summary.isNotBlank()) {
                Text(
                    resultEvent.summary,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                    fontSize = 12.sp,
                    fontFamily = FontFamily.Monospace,
                    maxLines = 10,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(start = 24.dp, top = 2.dp)
                )
            }
        }
    }
}

// ─── Tool Result (inline) ───────────────────────────────────────────────
@Composable
fun ToolResultInline(event: ToolResultEvent) {
    Row(
        modifier = Modifier.padding(start = 24.dp, end = 48.dp, top = 1.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            if (event.success) Icons.Default.CheckCircle else Icons.Default.Error,
            null,
            tint = if (event.success) MaterialTheme.colorScheme.primary.copy(alpha = 0.6f) else MaterialTheme.colorScheme.error.copy(alpha = 0.6f),
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
        Spacer(Modifier.height(2.dp))
        event.todos.forEach { todo ->
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 1.dp)) {
                Icon(
                    if (todo.status == "done") Icons.Default.Check else Icons.Default.Send,
                    null,
                    tint = if (todo.status == "done") MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(12.dp)
                )
                Spacer(Modifier.width(4.dp))
                Text(todo.text,
                    color = if (todo.status == "done") MaterialTheme.colorScheme.onSurface.copy(alpha = 0.4f) else MaterialTheme.colorScheme.onSurface,
                    fontSize = 12.sp)
            }
        }
    }
}

// ─── File Read Block ────────────────────────────────────────────────────
@Composable
fun FileReadBlock(event: FileReadEvent) {
    Column(modifier = Modifier.fillMaxWidth().padding(end = 48.dp, top = 2.dp)) {
        Text(event.path, color = MaterialTheme.colorScheme.primary.copy(alpha = 0.7f),
            fontSize = 11.sp, fontFamily = FontFamily.Monospace)
        Text(event.text.take(300), color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
            fontSize = 11.sp, fontFamily = FontFamily.Monospace, maxLines = 8, overflow = TextOverflow.Ellipsis)
    }
}

// ─── Streaming Text (no bubble, full width) ─────────────────────────────
@Composable
fun StreamingText(event: TextEvent) {
    val clipboardManager = LocalClipboardManager.current
    Column(
        modifier = Modifier.fillMaxWidth().padding(top = 8.dp, end = 8.dp)
    ) {
        Text(
            text = event.text,
            color = MaterialTheme.colorScheme.onSurface,
            fontSize = 15.sp,
            lineHeight = 22.sp
        )
        // Action row
        Row(
            modifier = Modifier.padding(top = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            IconButton(onClick = {
                clipboardManager.setText(AnnotatedString(event.text))
            }, modifier = Modifier.size(28.dp)) {
                Icon(Icons.Default.ContentCopy, "Copy", modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.35f))
            }
        }
    }
}

// ─── Response Block (final done event + stats bar) ──────────────────────
@Composable
fun ResponseBlock(event: DoneEvent) {
    val clipboardManager = LocalClipboardManager.current
    Column(modifier = Modifier.fillMaxWidth().padding(top = 2.dp, end = 8.dp)) {
        // Summary text (if different from TextEvent)
        if (event.summary.isNotBlank()) {
            Text(
                text = event.summary,
                color = MaterialTheme.colorScheme.onSurface,
                fontSize = 15.sp,
                lineHeight = 22.sp
            )
        }

        // Action row
        Row(
            modifier = Modifier.padding(top = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            IconButton(onClick = {
                clipboardManager.setText(AnnotatedString(event.summary))
            }, modifier = Modifier.size(28.dp)) {
                Icon(Icons.Default.ContentCopy, "Copy", modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.35f))
            }
            IconButton(onClick = { /* TODO: regenerate */ }, modifier = Modifier.size(28.dp)) {
                Icon(Icons.Default.Refresh, "Regenerate", modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.35f))
            }
            IconButton(onClick = { /* TODO: more options */ }, modifier = Modifier.size(28.dp)) {
                Icon(Icons.Default.MoreVert, "More", modifier = Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.35f))
            }
        }

        // Stats bar: ↑12,3K ↓2,6K ⚡113,9 tok/s ⏱23,1s
        if (event.total_tokens > 0) {
            Row(
                modifier = Modifier.padding(top = 2.dp),
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
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                    fontSize = 11.sp
                )
                Text(
                    "⚡${fmtD(event.tokens_per_sec)} tok/s",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                    fontSize = 11.sp
                )
                val seconds = event.elapsed_ms / 1000.0
                Text(
                    "⏱${"%.1f".format(seconds)}s",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
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
        shape = RoundedCornerShape(8.dp)
    ) {
        Row(modifier = Modifier.padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.Default.Warning, null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(16.dp))
            Spacer(Modifier.width(8.dp))
            Text(event.message, color = MaterialTheme.colorScheme.error, fontSize = 13.sp)
        }
    }
}

// ─── Model Chips Bar ────────────────────────────────────────────────────
@Composable
fun ModelChipsBar(models: List<String>, selected: String, onSelect: (String) -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        models.take(4).forEach { model ->
            val isSelected = model == selected
            val shortName = model.split("/").last().take(15)
            Surface(
                modifier = Modifier.clickable { onSelect(model) },
                shape = RoundedCornerShape(16.dp),
                color = if (isSelected) MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)
                else MaterialTheme.colorScheme.surfaceContainerHigh,
                border = if (isSelected) ButtonDefaults.outlinedButtonBorder(enabled = true) else null
            ) {
                Text(
                    shortName,
                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                    fontSize = 12.sp,
                    color = if (isSelected) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                    maxLines = 1, overflow = TextOverflow.Ellipsis
                )
            }
        }
        if (models.size > 4) {
            Text("+${models.size - 4}", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f),
                modifier = Modifier.padding(top = 5.dp))
        }
    }
}

// ─── Input Bar ──────────────────────────────────────────────────────────
@Composable
fun InputBar(
    input: String,
    onInputChange: (String) -> Unit,
    onSend: () -> Unit,
    isRunning: Boolean,
    focusRequester: FocusRequester
) {
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
                onValueChange = onInputChange,
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
                keyboardActions = KeyboardActions(onSend = onSend),
                maxLines = 5,
            )
            FilledIconButton(
                onClick = onSend,
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
                    CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp,
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
                } else {
                    Icon(Icons.Default.ArrowUpward, "Send", modifier = Modifier.size(22.dp))
                }
            }
        }
    }
}
