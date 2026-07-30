package com.brain.app.features.chat

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.theme.BrainColors

@Composable
fun EmptyState(
    onStartChat: (String) -> Unit,
    onOpenSettings: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    var isDropdownOpen by remember { mutableStateOf(false) }
    var customPath by remember { mutableStateOf("") }
    var isCustomMode by remember { mutableStateOf(false) }

    Box(
        modifier = modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            // Logo
            Box(
                modifier = Modifier
                    .size(64.dp)
                    .clip(RoundedCornerShape(16.dp))
                    .background(
                        Brush.linearGradient(
                            colors = listOf(BrainColors.accentMain100, BrainColors.accentMain200)
                        )
                    ),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    Icons.Default.ChatBubbleOutline,
                    contentDescription = null,
                    tint = BrainColors.oncolor100,
                    modifier = Modifier.size(32.dp)
                )
            }

            Spacer(modifier = Modifier.height(24.dp))

            // Title
            Text(
                text = "New Chat",
                fontSize = 20.sp,
                fontWeight = FontWeight.SemiBold,
                color = BrainColors.text100
            )

            Spacer(modifier = Modifier.height(8.dp))

            // Description
            Text(
                text = "Start a conversation with your AI agent",
                fontSize = 14.sp,
                color = BrainColors.text400,
                textAlign = TextAlign.Center
            )

            Spacer(modifier = Modifier.height(32.dp))

            // Directory Selector
            Column(modifier = Modifier.fillMaxWidth()) {
                Text(
                    text = "WORKING DIRECTORY",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = BrainColors.text400,
                    letterSpacing = 1.sp,
                    modifier = Modifier.padding(bottom = 8.dp)
                )

                if (isCustomMode) {
                    // Custom path input
                    Column {
                        OutlinedTextField(
                            value = customPath,
                            onValueChange = { customPath = it },
                            modifier = Modifier.fillMaxWidth(),
                            placeholder = { Text("Enter absolute path", color = BrainColors.text500) },
                            leadingIcon = { Icon(Icons.Default.Folder, null, tint = BrainColors.text400, modifier = Modifier.size(20.dp)) },
                            textStyle = MaterialTheme.typography.bodyMedium.copy(color = BrainColors.text100),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedBorderColor = BrainColors.accentMain100.copy(alpha = 0.5f),
                                unfocusedBorderColor = BrainColors.border300.copy(alpha = 0.3f),
                                focusedContainerColor = BrainColors.bg200,
                                unfocusedContainerColor = BrainColors.bg200
                            ),
                            shape = RoundedCornerShape(8.dp),
                            singleLine = true
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "← Back to directory list",
                            fontSize = 12.sp,
                            color = BrainColors.text400,
                            modifier = Modifier.clickable { isCustomMode = false }
                        )
                    }
                } else {
                    // Directory dropdown
                    Box {
                        OutlinedTextField(
                            value = "",
                            onValueChange = {},
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { isDropdownOpen = !isDropdownOpen },
                            enabled = false,
                            placeholder = { Text("Select directory", color = BrainColors.text500) },
                            leadingIcon = { Icon(Icons.Default.Folder, null, tint = BrainColors.text400, modifier = Modifier.size(20.dp)) },
                            trailingIcon = {
                                Icon(
                                    if (isDropdownOpen) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                                    null,
                                    tint = BrainColors.text400,
                                    modifier = Modifier.size(20.dp)
                                )
                            },
                            textStyle = MaterialTheme.typography.bodyMedium.copy(color = BrainColors.text100),
                            colors = OutlinedTextFieldDefaults.colors(
                                disabledBorderColor = BrainColors.border300.copy(alpha = 0.3f),
                                disabledContainerColor = BrainColors.bg200,
                                disabledTextColor = BrainColors.text100,
                                disabledPlaceholderColor = BrainColors.text500,
                                disabledLeadingIconColor = BrainColors.text400,
                                disabledTrailingIconColor = BrainColors.text400
                            ),
                            shape = RoundedCornerShape(8.dp),
                            singleLine = true
                        )

                        androidx.compose.animation.AnimatedVisibility(
                            visible = isDropdownOpen,
                            enter = fadeIn(),
                            exit = fadeOut()
                        ) {
                            Card(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(top = 4.dp),
                                colors = CardDefaults.cardColors(containerColor = BrainColors.bg300),
                                shape = RoundedCornerShape(12.dp)
                            ) {
                                Column {
                                    // Custom path option
                                    Row(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clickable {
                                                isCustomMode = true
                                                isDropdownOpen = false
                                            }
                                            .padding(horizontal = 12.dp, vertical = 10.dp),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Icon(
                                            Icons.Default.Add,
                                            null,
                                            tint = BrainColors.text400,
                                            modifier = Modifier.size(16.dp)
                                        )
                                        Spacer(modifier = Modifier.width(8.dp))
                                        Text("Enter custom path", fontSize = 14.sp, color = BrainColors.text400)
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(24.dp))

            // Start Button
            Button(
                onClick = {
                    val path = if (isCustomMode) customPath.trim() else ""
                    if (path.isNotBlank()) onStartChat(path)
                },
                modifier = Modifier.fillMaxWidth(),
                colors = ButtonDefaults.buttonColors(
                    containerColor = BrainColors.accentMain100,
                    contentColor = BrainColors.oncolor100,
                    disabledContainerColor = BrainColors.accentMain100.copy(alpha = 0.5f),
                    disabledContentColor = BrainColors.oncolor100.copy(alpha = 0.5f)
                ),
                shape = RoundedCornerShape(8.dp),
                enabled = if (isCustomMode) customPath.isNotBlank() else true
            ) {
                Text("Start Conversation", fontWeight = FontWeight.Medium)
            }

            Spacer(modifier = Modifier.height(16.dp))

            // Hint
            Text(
                text = "Or just type a message below",
                fontSize = 12.sp,
                color = BrainColors.text500,
                textAlign = TextAlign.Center
            )
        }
    }
}
