package com.brain.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.ChevronDown
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.SmartToy
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.graphics.vector.ImageVector
import com.brain.app.ui.theme.BrainColors

// ═══════════════════════════════════════════════════════════════════════════
// EmptyChatScreen — 1:1 visual replica of OpenCodeUI
// Pure UI demo, no backend logic
// ═══════════════════════════════════════════════════════════════════════════

@Composable
fun EmptyChatScreen(
    onSettingsClick: () -> Unit = {},
) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(BrainColors.Bg000)
            .statusBarsPadding()
            .navigationBarsPadding()
            .imePadding()
    ) {
        Column(modifier = Modifier.fillMaxSize()) {

            // ── Header ──────────────────────────────────────────────────
            ChatHeader(onSettingsClick = onSettingsClick)

            // ── Center content ──────────────────────────────────────────
            Box(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState()),
                contentAlignment = Alignment.Center
            ) {
                EmptyStateContent()
            }

            // ── Input area ──────────────────────────────────────────────
            ChatInputArea()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HEADER — matches OpenCodeUI Header.tsx
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun ChatHeader(onSettingsClick: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(48.dp)
            .background(BrainColors.Bg000)
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // Title
        Text(
            text = "New chat",
            color = BrainColors.Text200,
            fontSize = 15.sp,
            fontWeight = FontWeight.Medium,
            modifier = Modifier.weight(1f)
        )

        // Settings icon
        IconButton(
            onClick = onSettingsClick,
            modifier = Modifier.size(32.dp),
            colors = IconButtonDefaults.iconButtonColors(
                containerColor = BrainColors.Bg300,
                contentColor = BrainColors.Text300,
            )
        ) {
            Icon(
                Icons.Default.Settings,
                contentDescription = "Settings",
                modifier = Modifier.size(16.dp)
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EMPTY STATE — matches OpenCodeUI EmptyState.tsx
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun EmptyStateContent() {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        // ── Gradient logo icon ──────────────────────────────────────────
        Box(
            modifier = Modifier
                .size(64.dp)
                .clip(RoundedCornerShape(16.dp))
                .background(
                    Brush.linearGradient(
                        colors = listOf(
                            BrainColors.AccentMain100,
                            BrainColors.AccentMain200,
                        )
                    )
                ),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                Icons.Default.Chat,
                contentDescription = null,
                tint = BrainColors.AlwaysWhite,
                modifier = Modifier.size(32.dp)
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        // ── Title ───────────────────────────────────────────────────────
        Text(
            text = "How can I help you?",
            color = BrainColors.Text100,
            fontSize = 22.sp,
            fontWeight = FontWeight.SemiBold,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(8.dp))

        // ── Description ─────────────────────────────────────────────────
        Text(
            text = "Choose a directory to start, or just type a message.",
            color = BrainColors.Text400,
            fontSize = 14.sp,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(32.dp))

        // ── Directory selector ──────────────────────────────────────────
        DirectorySelector()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DIRECTORY SELECTOR — matches OpenCodeUI dropdown
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun DirectorySelector() {
    var expanded by remember { mutableStateOf(false) }

    Column {
        // Trigger button
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(12.dp))
                .background(BrainColors.Bg200)
                .border(1.dp, BrainColors.Border300.copy(alpha = 0.3f), RoundedCornerShape(12.dp))
                .clickable { expanded = !expanded }
                .padding(horizontal = 12.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                Icons.Default.Folder,
                contentDescription = null,
                tint = BrainColors.Text400,
                modifier = Modifier.size(16.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = "/home/user/project",
                color = BrainColors.Text100,
                fontSize = 14.sp,
                modifier = Modifier.weight(1f),
                maxLines = 1
            )
            Icon(
                Icons.Default.ChevronDown,
                contentDescription = null,
                tint = BrainColors.Text400,
                modifier = Modifier.size(16.dp)
            )
        }

        // Dropdown
        AnimatedVisibility(
            visible = expanded,
            enter = expandVertically() + fadeIn(),
            exit = shrinkVertically() + fadeOut()
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 4.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(BrainColors.Bg300)
                    .border(1.dp, BrainColors.Border200.copy(alpha = 0.6f), RoundedCornerShape(12.dp))
            ) {
                // Current directory
                DropdownItem(
                    text = "/home/user/project",
                    label = "current",
                    iconTint = BrainColors.AccentMain100,
                    textColor = BrainColors.Text200,
                    onClick = { expanded = false }
                )

                // Divider
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(1.dp)
                        .background(BrainColors.Border300.copy(alpha = 0.2f))
                )

                // Custom path
                DropdownItem(
                    text = "Enter custom path…",
                    label = null,
                    iconTint = BrainColors.Text500,
                    textColor = BrainColors.Text400,
                    onClick = { expanded = false }
                )
            }
        }
    }
}

@Composable
private fun DropdownItem(
    text: String,
    label: String?,
    iconTint: Color,
    textColor: Color,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            Icons.Default.Folder,
            contentDescription = null,
            tint = iconTint,
            modifier = Modifier.size(16.dp)
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = text,
            color = textColor,
            fontSize = 14.sp,
            maxLines = 1,
            modifier = Modifier.weight(1f)
        )
        if (label != null) {
            Text(
                text = label,
                color = BrainColors.Text500,
                fontSize = 12.sp
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// INPUT AREA — matches OpenCodeUI InputBox + InputToolbar
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun ChatInputArea() {
    var text by remember { mutableStateOf("") }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 12.dp)
    ) {
        // ── Textarea container ──────────────────────────────────────────
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(20.dp))
                .background(BrainColors.Bg200)
                .border(1.dp, BrainColors.Border200.copy(alpha = 0.4f), RoundedCornerShape(20.dp))
        ) {
            // Text input
            BasicTextField(
                value = text,
                onValueChange = { text = it },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 14.dp),
                textStyle = TextStyle(
                    color = BrainColors.Text100,
                    fontSize = 14.sp,
                    lineHeight = 20.sp
                ),
                cursorBrush = SolidColor(BrainColors.AccentMain100),
                decorationBox = { innerField ->
                    if (text.isEmpty()) {
                        Text(
                            text = "Type a message…",
                            color = BrainColors.Text500,
                            fontSize = 14.sp
                        )
                    }
                    innerField()
                },
                maxLines = 5
            )

            // ── Toolbar ─────────────────────────────────────────────────
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 8.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                // Left: model + agent selectors
                Row(
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Model selector chip
                    ToolbarChip(icon = Icons.Default.SmartToy, label = "mimo-v2.5")

                    // Agent chip
                    ToolbarChip(icon = Icons.Default.SmartToy, label = "build")
                }

                // Right: attach + send
                Row(
                    horizontalArrangement = Arrangement.spacedBy(2.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    IconButton(
                        onClick = { /* no-op demo */ },
                        modifier = Modifier.size(32.dp),
                        colors = IconButtonDefaults.iconButtonColors(
                            containerColor = BrainColors.Bg300,
                            contentColor = BrainColors.Text300,
                        )
                    ) {
                        Icon(
                            Icons.Default.AttachFile,
                            contentDescription = "Attach",
                            modifier = Modifier.size(16.dp)
                        )
                    }

                    // Send button (accent green)
                    IconButton(
                        onClick = { /* no-op demo */ },
                        modifier = Modifier.size(32.dp),
                        colors = IconButtonDefaults.iconButtonColors(
                            containerColor = BrainColors.AccentMain100,
                            contentColor = BrainColors.AlwaysWhite,
                        )
                    ) {
                        Icon(
                            Icons.AutoMirrored.Filled.Send,
                            contentDescription = "Send",
                            modifier = Modifier.size(16.dp)
                        )
                    }
                }
            }
        }

        // ── Hint text ───────────────────────────────────────────────────
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Press Enter to send · Shift+Enter for new line",
            color = BrainColors.Text500,
            fontSize = 11.sp,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth()
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TOOLBAR CHIP — small selector pill (model/agent)
// ═══════════════════════════════════════════════════════════════════════════

@Composable
private fun ToolbarChip(
    icon: ImageVector,
    label: String,
) {
    Row(
        modifier = Modifier
            .clip(RoundedCornerShape(8.dp))
            .background(BrainColors.Bg300)
            .clickable { /* no-op demo */ }
            .padding(horizontal = 8.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = BrainColors.Text400,
            modifier = Modifier.size(12.dp)
        )
        Text(
            text = label,
            color = BrainColors.Text300,
            fontSize = 12.sp
        )
        Icon(
            Icons.Default.ChevronDown,
            contentDescription = null,
            tint = BrainColors.Text400,
            modifier = Modifier.size(12.dp)
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PREVIEW
// ═══════════════════════════════════════════════════════════════════════════

@Composable
@androidx.compose.ui.tooling.preview.Preview
private fun EmptyChatScreenPreview() {
    com.brain.app.ui.theme.BrainTheme {
        EmptyChatScreen()
    }
}
