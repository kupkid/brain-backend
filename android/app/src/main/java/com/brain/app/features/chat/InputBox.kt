package com.brain.app.features.chat

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.theme.BrainColors
import com.brain.app.theme.BrainShapes

@Composable
fun InputBox(
    value: String,
    onValueChange: (String) -> Unit,
    onSend: () -> Unit,
    onStop: () -> Unit,
    isRunning: Boolean,
    selectedModel: String,
    onModelClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val focusRequester = remember { FocusRequester() }
    val focusManager = LocalFocusManager.current

    Column(
        modifier = modifier
            .fillMaxWidth()
            .background(BrainColors.bg000)
            .padding(horizontal = 16.dp, vertical = 8.dp)
    ) {
        // Toolbar row
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Model selector chip
            ModelInputChip(
                model = selectedModel,
                onClick = onModelClick
            )

            Spacer(Modifier.weight(1f))

            // Attach button
            IconButton(
                onClick = { },
                modifier = Modifier.size(32.dp)
            ) {
                Icon(
                    Icons.Default.AttachFile,
                    contentDescription = "Attach",
                    modifier = Modifier.size(18.dp),
                    tint = BrainColors.text400
                )
            }
        }

        Spacer(Modifier.height(4.dp))

        // Input container
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(BrainShapes.inputField))
                .background(BrainColors.bg300)
                .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.Bottom
            ) {
                BasicTextField(
                    value = value,
                    onValueChange = onValueChange,
                    modifier = Modifier
                        .weight(1f)
                        .focusRequester(focusRequester),
                    textStyle = MaterialTheme.typography.bodyMedium.copy(
                        color = BrainColors.text100,
                        fontSize = 14.sp
                    ),
                    cursorBrush = SolidColor(BrainColors.accentMain100),
                    decorationBox = { inner ->
                        if (value.isEmpty()) {
                            Text(
                                text = "Type a message...",
                                color = BrainColors.text400,
                                fontSize = 14.sp
                            )
                        }
                        inner()
                    },
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                    keyboardActions = KeyboardActions(onSend = {
                        if (value.isNotBlank()) {
                            onSend()
                            onValueChange("")
                        }
                    }),
                    maxLines = 8
                )

                Spacer(Modifier.width(8.dp))

                // Send / Stop button
                if (isRunning) {
                    IconButton(
                        onClick = onStop,
                        modifier = Modifier
                            .size(36.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(BrainColors.danger100)
                    ) {
                        Icon(
                            Icons.Default.Stop,
                            contentDescription = "Stop",
                            modifier = Modifier.size(18.dp),
                            tint = Color.White
                        )
                    }
                } else {
                    IconButton(
                        onClick = {
                            if (value.isNotBlank()) {
                                onSend()
                                onValueChange("")
                            }
                        },
                        enabled = value.isNotBlank(),
                        modifier = Modifier
                            .size(36.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(
                                if (value.isNotBlank()) BrainColors.accentMain100
                                else BrainColors.bg400
                            )
                    ) {
                        Icon(
                            Icons.AutoMirrored.Filled.Send,
                            contentDescription = "Send",
                            modifier = Modifier.size(18.dp),
                            tint = if (value.isNotBlank()) Color.White else BrainColors.text500
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ModelInputChip(
    model: String,
    onClick: () -> Unit
) {
    val displayName = model.split("/").lastOrNull()?.take(15) ?: model
    Surface(
        modifier = Modifier
            .clip(RoundedCornerShape(BrainShapes.full))
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(BrainShapes.full),
        color = BrainColors.bg400
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Icon(
                Icons.Default.SmartToy,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = BrainColors.accentMain100
            )
            Text(
                text = displayName.ifBlank { "Select model" },
                color = BrainColors.text200,
                fontSize = 11.sp,
                maxLines = 1
            )
            Icon(
                Icons.Default.ExpandMore,
                contentDescription = null,
                modifier = Modifier.size(12.dp),
                tint = BrainColors.text400
            )
        }
    }
}
