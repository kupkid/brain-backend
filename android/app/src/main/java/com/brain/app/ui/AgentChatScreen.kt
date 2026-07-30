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
import com.brain.app.ui.theme.AppShapes
import com.brain.app.ui.theme.BrainColors

// ═══════════════════════════════════════════════════════════════════════════
// BUBBLE POSITION — iMessage-style grouped corners
// ═══════════════════════════════════════════════════════════════════════════

enum class BubblePosition { SINGLE, FIRST, MIDDLE, LAST }

enum class BubbleRole { USER, ASSISTANT, ACTIVITY }

fun getBubblePosition(index: Int, total: Int): BubblePosition = when {
    total == 1 -> BubblePosition.SINGLE
    index == 0 -> BubblePosition.FIRST
    index == total - 1 -> BubblePosition.LAST
    else -> BubblePosition.MIDDLE
}

@Composable
fun bubbleShape(position: BubblePosition, role: BubbleRole): RoundedCornerShape {
    val large = 20.dp
    val small = 6.dp
    val isLeft = role != BubbleRole.USER
    return when (position) {
        BubblePosition.SINGLE -> RoundedCornerShape(large)
        BubblePosition.FIRST -> if (isLeft)
            RoundedCornerShape(large, large, large, small)
        else
            RoundedCornerShape(large, large, small, large)
        BubblePosition.MIDDLE -> if (isLeft)
            RoundedCornerShape(small, large, large, small)
        else
            RoundedCornerShape(large, small, small, large)
        BubblePosition.LAST -> if (isLeft)
            RoundedCornerShape(small, large, large, large)
        else
            RoundedCornerShape(large, small, large, large)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SHIMMER — for loading states
// ═══════════════════════════════════════════════════════════════════════════

@Composable
fun shimmerBrush(): Brush {
    val colors = listOf(BrainColors.Bg300, BrainColors.Bg100, BrainColors.Bg300)
    val transition = rememberInfiniteTransition(label = "sh")
    val translate = transition.animateFloat(
        0f, 1000f,
        infiniteRepeatable(tween(1200, easing = FastOutSlowInEasing), RepeatMode.Restart),
        label = "sh"
    )
    return Brush.linearGradient(colors, start = Offset.Zero, end = Offset(translate.value, translate.value))
}

// ═══════════════════════════════════════════════════════════════════════════
// ACTIVITY PILLS — tool call indicators
// ═══════════════════════════════════════════════════════════════════════════

enum class ActivityType { REASONING, SHELL, FILE, SEARCH, TODO, BROWSER, OTHER }

fun classifyTool(name: String): ActivityType = when {
    name.contains("shell", true) || name.contains("exec", true) -> ActivityType.SHELL
    name.contains("file", true) || name.contains("read", true) || name.contains("write", true) -> ActivityType.FILE
    name.contains("grep", true) || name.contains("search", true) -> ActivityType.SEARCH
    name.contains("todo", true) -> ActivityType.TODO
    name.contains("browser", true) -> ActivityType.BROWSER
    name.contains("list", true) || name.contains("dir", true) -> ActivityType.FILE
    else -> ActivityType.OTHER
}

@Composable
fun activityIcon(type: ActivityType) = when (type) {
    ActivityType.REASONING -> Icons.Default.Lightbulb
    ActivityType.SHELL -> Icons.Default.Terminal
    ActivityType.FILE -> Icons.Default.Description
    ActivityType.SEARCH -> Icons.Default.Search
    ActivityType.TODO -> Icons.Default.Checklist
    ActivityType.BROWSER -> Icons.Default.Language
    ActivityType.OTHER -> Icons.Default.Build
}

@Composable
fun ActivityPill(
    toolName: String,
    isRunning: Boolean,
    modifier: Modifier = Modifier,
) {
    val type = classifyTool(toolName)
    val icon = activityIcon(type)
    val color = when (type) {
        ActivityType.SHELL -> BrainColors.Success100
        ActivityType.FILE -> BrainColors.Info100
        ActivityType.SEARCH -> BrainColors.Warning100
        ActivityType.TODO -> BrainColors.AccentSecondary
        ActivityType.BROWSER -> BrainColors.AccentMain200
        ActivityType.REASONING -> BrainColors.Warning100
        ActivityType.OTHER -> BrainColors.Text300
    }

    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(20.dp),
        color = color.copy(alpha = 0.12f),
        contentColor = color
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            if (isRunning) {
                Box(modifier = Modifier.size(14.dp).clip(RoundedCornerShape(4.dp)).background(shimmerBrush()))
            } else {
                Icon(icon, null, modifier = Modifier.size(14.dp))
            }
            Text(
                toolName,
                fontSize = 12.sp,
                fontWeight = FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN CHAT SCREEN
// ═══════════════════════════════════════════════════════════════════════════

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
        if (events.isNotEmpty()) listState.animateScrollToItem(events.size - 1)
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
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent)
            )
        },
        containerColor = MaterialTheme.colorScheme.background
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            if (events.isEmpty() && !isRunning) {
                EmptyState()
            } else {
                ChatMessageList(events, listState)
            }
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

// ═══════════════════════════════════════════════════════════════════════════
// EMPTY STATE
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun EmptyState() {
    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.padding(horizontal = 32.dp)
        ) {
            Box(
                modifier = Modifier.size(72.dp).clip(CircleShape)
                    .background(BrainColors.AccentMain100.copy(alpha = 0.08f)),
                contentAlignment = Alignment.Center
            ) {
                Icon(Icons.Default.AutoAwesome, null, Modifier.size(36.dp),
                    tint = BrainColors.AccentMain100.copy(alpha = 0.3f))
            }
            Spacer(Modifier.height(20.dp))
            Text("Чем могу помочь?", fontSize = 22.sp, fontWeight = FontWeight.SemiBold,
                color = BrainColors.Text100)
            Spacer(Modifier.height(8.dp))
            Text("Задайте вопрос или поручите задачу", fontSize = 14.sp,
                color = BrainColors.Text400)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MESSAGE LIST — grouped by role with smart corners
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun ChatMessageList(events: List<AgentEvent>, listState: androidx.compose.foundation.lazy.LazyListState) {
    val groups = remember(events) { buildEventGroups(events) }

    LazyColumn(
        state = listState,
        modifier = Modifier.weight(1f).fillMaxWidth().padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
        contentPadding = PaddingValues(vertical = 8.dp)
    ) {
        items(groups.size) { groupIdx ->
            val group = groups[groupIdx]
            when (group) {
                is EventGroup.UserMessages -> {
                    val texts = group.texts
                    items(texts.size) { i ->
                        val pos = getBubblePosition(i, texts.size)
                        UserBubble(texts[i], pos)
                    }
                }
                is EventGroup.Activity -> {
                    ActivityPill(group.toolName, group.isRunning,
                        modifier = Modifier.padding(top = 4.dp, end = 48.dp))
                    group.result?.let { result ->
                        ToolResultInline(result)
                    }
                }
                is EventGroup.Todo -> TodoProgress(group.event)
                is EventGroup.FileRead -> FileReadBlock(group.event)
                is EventGroup.Streaming -> StreamingText(group.event)
                is EventGroup.Response -> ResponseBlock(group.event)
                is EventGroup.Error -> ErrorBlock(group.event)
            }
        }
    }
}

// Event grouping logic
sealed class EventGroup {
    data class UserMessages(val texts: List<String>) : EventGroup()
    data class Activity(val toolName: String, val isRunning: Boolean, val result: ToolResultEvent? = null) : EventGroup()
    data class Todo(val event: TodoUpdateEvent) : EventGroup()
    data class FileRead(val event: FileReadEvent) : EventGroup()
    data class Streaming(val event: TextEvent) : EventGroup()
    data class Response(val event: DoneEvent) : EventGroup()
    data class Error(val event: ErrorEvent) : EventGroup()
}

private fun buildEventGroups(events: List<AgentEvent>): List<EventGroup> {
    val groups = mutableListOf<EventGroup>()
    var pendingUserTexts = mutableListOf<String>()

    fun flushUser() {
        if (pendingUserTexts.isNotEmpty()) {
            groups.add(EventGroup.UserMessages(pendingUserTexts.toList()))
            pendingUserTexts = mutableListOf()
        }
    }

    var i = 0
    while (i < events.size) {
        val ev = events[i]
        when (ev) {
            is ThoughtEvent -> {
                pendingUserTexts.add(ev.text)
            }
            is ToolCallEvent -> {
                flushUser()
                val toolName = ev.tool
                val callId = ev.call_id
                var result: ToolResultEvent? = null
                var j = i + 1
                while (j < events.size) {
                    val next = events[j]
                    if (next is ToolResultEvent && next.call_id == callId) {
                        result = next
                        j++
                        break
                    }
                    if (next is ToolCallEvent) break
                    j++
                }
                groups.add(EventGroup.Activity(toolName, result == null, result))
                i = j - 1
            }
            is ToolResultEvent -> { /* handled with ToolCall */ }
            is TodoUpdateEvent -> { flushUser(); groups.add(EventGroup.Todo(ev)) }
            is FileReadEvent -> { flushUser(); groups.add(EventGroup.FileRead(ev)) }
            is TextEvent -> { flushUser(); groups.add(EventGroup.Streaming(ev)) }
            is DoneEvent -> { flushUser(); groups.add(EventGroup.Response(ev)) }
            is ErrorEvent -> { flushUser(); groups.add(EventGroup.Error(ev)) }
        }
        i++
    }
    flushUser()
    return groups
}

// ═══════════════════════════════════════════════════════════════════════════
// USER BUBBLE — right-aligned with smart corners
// ═══════════════════════════════════════════════════════════════════════════

@Composable
fun UserBubble(text: String, position: BubblePosition = BubblePosition.SINGLE) {
    val shape = bubbleShape(position, BubbleRole.USER)
    val verticalPadding = when (position) {
        BubblePosition.FIRST -> PaddingValues(start = 48.dp, end = 0.dp, top = 8.dp, bottom = 1.dp)
        BubblePosition.MIDDLE -> PaddingValues(start = 48.dp, end = 0.dp, top = 1.dp, bottom = 1.dp)
        BubblePosition.LAST -> PaddingValues(start = 48.dp, end = 0.dp, top = 1.dp, bottom = 8.dp)
        BubblePosition.SINGLE -> PaddingValues(start = 48.dp, end = 0.dp, top = 8.dp, bottom = 8.dp)
    }
    Row(
        modifier = Modifier.fillMaxWidth().padding(verticalPadding),
        horizontalArrangement = Arrangement.End
    ) {
        Surface(
            shape = shape,
            color = BrainColors.AccentMain100,
            modifier = Modifier.widthIn(max = 320.dp)
        ) {
            Text(
                text = text, color = BrainColors.OnColor,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                fontSize = 15.sp, lineHeight = 22.sp
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TOOL RESULT — inline green/red badge
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// TODO PROGRESS
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// FILE READ BLOCK
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// STREAMING TEXT
// ═══════════════════════════════════════════════════════════════════════════

@Composable
fun StreamingText(event: TextEvent) {
    val clipboardManager = LocalClipboardManager.current
    Column(modifier = Modifier.fillMaxWidth().padding(top = 12.dp, end = 8.dp)) {
        Text(event.text, color = MaterialTheme.colorScheme.onSurface, fontSize = 15.sp, lineHeight = 23.sp)
        Row(modifier = Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            IconButton(onClick = { clipboardManager.setText(AnnotatedString(event.text)) }, modifier = Modifier.size(28.dp)) {
                Icon(Icons.Default.ContentCopy, "Copy", Modifier.size(16.dp),
                    tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESPONSE BLOCK + STATS
// ═══════════════════════════════════════════════════════════════════════════

@Composable
fun ResponseBlock(event: DoneEvent) {
    val clipboardManager = LocalClipboardManager.current
    Column(modifier = Modifier.fillMaxWidth().padding(top = 4.dp, end = 8.dp)) {
        if (event.summary.isNotBlank()) {
            Text(event.summary, color = MaterialTheme.colorScheme.onSurface, fontSize = 15.sp, lineHeight = 23.sp)
        }
        Row(modifier = Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
            AssistChip(onClick = { clipboardManager.setText(AnnotatedString(event.summary)) },
                label = { Text("Копировать", fontSize = 11.sp) },
                leadingIcon = { Icon(Icons.Default.ContentCopy, null, Modifier.size(14.dp)) },
                modifier = Modifier.height(28.dp), shape = RoundedCornerShape(8.dp),
                colors = AssistChipDefaults.assistChipColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh))
            AssistChip(onClick = { },
                label = { Text("Ещё раз", fontSize = 11.sp) },
                leadingIcon = { Icon(Icons.Default.Refresh, null, Modifier.size(14.dp)) },
                modifier = Modifier.height(28.dp), shape = RoundedCornerShape(8.dp),
                colors = AssistChipDefaults.assistChipColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh))
        }
        if (event.total_tokens > 0) {
            Row(modifier = Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                val fmt = { n: Int -> if (n >= 1000) "${"%.1f".format(n / 1000f)}K" else "$n" }
                Text("↑${fmt(event.tokens_input)} ↓${fmt(event.tokens_output)}",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f), fontSize = 11.sp)
                val tps = event.tokens_per_sec
                if (tps > 0) Text("⚡${"%.1f".format(tps)} tok/s",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f), fontSize = 11.sp)
                val seconds = event.elapsed_ms / 1000.0
                Text("⏱${"%.1f".format(seconds)}s",
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.25f), fontSize = 11.sp)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ERROR BLOCK
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// CHAT INPUT — pill shape with model selector
// ═══════════════════════════════════════════════════════════════════════════

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
            Surface(
                shape = AppShapes.InputField,
                color = MaterialTheme.colorScheme.surfaceContainer,
                modifier = Modifier.fillMaxWidth()
            ) {
                Column {
                    OutlinedTextField(
                        value = input, onValueChange = onInputChange,
                        modifier = Modifier.fillMaxWidth().focusRequester(focusRequester),
                        placeholder = { Text("Сообщение...",
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f), fontSize = 15.sp) },
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
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
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
                        } else Spacer(Modifier.height(1.dp))

                        FilledIconButton(
                            onClick = onSend,
                            enabled = input.isNotBlank() && !isRunning,
                            modifier = Modifier.size(40.dp),
                            shape = AppShapes.ButtonSquared,
                            colors = IconButtonDefaults.filledIconButtonColors(
                                containerColor = MaterialTheme.colorScheme.primary,
                                contentColor = MaterialTheme.colorScheme.onPrimary,
                                disabledContainerColor = MaterialTheme.colorScheme.surfaceVariant,
                                disabledContentColor = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.2f)
                            )
                        ) {
                            if (isRunning) CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.3f))
                            else Icon(Icons.Default.ArrowUpward, "Send", Modifier.size(20.dp))
                        }
                    }
                }
            }

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
                            Surface(
                                modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp))
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
                                    Text(model.split("/").last(), fontSize = 14.sp,
                                        color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                                        modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
