package com.brain.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Code
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
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
    onSendTask: (String) -> Unit,
    onStop: () -> Unit
) {
    var input by remember { mutableStateOf("") }
    val listState = rememberLazyListState()

    LaunchedEffect(events.size) {
        if (events.isNotEmpty()) {
            listState.animateScrollToItem(events.size - 1)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Brain Agent") },
                actions = {
                    if (isRunning) {
                        IconButton(onClick = onStop) {
                            Icon(Icons.Default.Close, "Stop", tint = MaterialTheme.colorScheme.error)
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color.Black,
                    titleContentColor = Color.White,
                )
            )
        },
        bottomBar = {
            Surface(
                color = Color(0xFF0A0A0A),
                tonalElevation = 0.dp
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 12.dp, vertical = 8.dp)
                        .imePadding(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    OutlinedTextField(
                        value = input,
                        onValueChange = { input = it },
                        modifier = Modifier.weight(1f),
                        placeholder = { Text("Задача...", color = Color(0xFF666666)) },
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = MaterialTheme.colorScheme.primary,
                            unfocusedBorderColor = Color(0xFF333333),
                            cursorColor = MaterialTheme.colorScheme.primary,
                            focusedTextColor = Color.White,
                            unfocusedTextColor = Color.White,
                        ),
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                        keyboardActions = KeyboardActions(
                            onSend = {
                                if (input.isNotBlank() && !isRunning) {
                                    onSendTask(input)
                                    input = ""
                                }
                            }
                        ),
                        singleLine = false,
                        maxLines = 4,
                    )
                    Spacer(Modifier.width(8.dp))
                    IconButton(
                        onClick = {
                            if (input.isNotBlank() && !isRunning) {
                                onSendTask(input)
                                input = ""
                            }
                        },
                        enabled = input.isNotBlank() && !isRunning
                    ) {
                        Icon(
                            Icons.Default.Send,
                            "Send",
                            tint = if (input.isNotBlank() && !isRunning)
                                MaterialTheme.colorScheme.primary
                            else Color(0xFF444444)
                        )
                    }
                }
            }
        },
        containerColor = Color.Black
    ) { padding ->
        if (events.isEmpty() && !isRunning) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    "Введите задачу",
                    color = Color(0xFF444444),
                    fontSize = 16.sp
                )
            }
        } else {
            LazyColumn(
                state = listState,
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = 12.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
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
    }
}

@Composable
fun ThoughtBubble(event: ThoughtEvent) {
    Text(
        text = event.text,
        color = Color(0xFF888888),
        fontStyle = FontStyle.Italic,
        fontSize = 14.sp,
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
        colors = CardDefaults.cardColors(containerColor = Color(0xFF111111)),
        shape = RoundedCornerShape(8.dp)
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
                        color = Color(0xFF777777),
                        fontSize = 12.sp,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                }
            }
            Text(
                text = event.call_id,
                color = Color(0xFF555555),
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
            tint = if (event.success) Color(0xFF66BB6A) else Color(0xFFEF5350),
            modifier = Modifier.size(14.dp)
        )
        Spacer(Modifier.width(6.dp))
        Text(
            text = event.summary.take(120),
            color = if (event.success) Color(0xFF999999) else Color(0xFFEF5350),
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
        colors = CardDefaults.cardColors(containerColor = Color(0xFF0D1117)),
        shape = RoundedCornerShape(8.dp)
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
                    trackColor = Color(0xFF222222),
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
                        tint = if (todo.status == "done") Color(0xFF66BB6A) else Color(0xFF666666),
                        modifier = Modifier.size(14.dp)
                    )
                    Spacer(Modifier.width(6.dp))
                    Text(
                        text = todo.text,
                        color = if (todo.status == "done") Color(0xFF777777) else Color.White,
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
        colors = CardDefaults.cardColors(containerColor = Color(0xFF0A0F14)),
        shape = RoundedCornerShape(8.dp)
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
                color = Color(0xFF999999),
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
        colors = CardDefaults.cardColors(containerColor = Color(0xFF0A2A0A)),
        shape = RoundedCornerShape(8.dp)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = event.summary,
                color = Color(0xFF66BB6A),
                fontWeight = FontWeight.Medium,
                fontSize = 14.sp
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = "${event.total_tokens} tokens · ${event.total_calls} tools",
                color = Color(0xFF666666),
                fontSize = 12.sp
            )
        }
    }
}

@Composable
fun ErrorCard(event: ErrorEvent) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Color(0xFF2A0A0A)),
        shape = RoundedCornerShape(8.dp)
    ) {
        Text(
            text = event.message,
            color = Color(0xFFEF5350),
            modifier = Modifier.padding(12.dp),
            fontSize = 14.sp
        )
    }
}
