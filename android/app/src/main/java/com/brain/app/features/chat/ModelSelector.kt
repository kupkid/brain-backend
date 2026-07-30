package com.brain.app.features.chat

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.theme.BrainColors

@Composable
fun ModelSelector(
    models: List<String>,
    selected: String,
    onSelect: (String) -> Unit,
    onDismiss: () -> Unit
) {
    var searchQuery by remember { mutableStateOf("") }

    val filteredModels = remember(models, searchQuery) {
        if (searchQuery.isBlank()) models
        else models.filter { it.contains(searchQuery, ignoreCase = true) }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = BrainColors.bg200,
        title = {
            Text("Select Model", color = BrainColors.text100)
        },
        text = {
            Column {
                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = { searchQuery = it },
                    modifier = Modifier.fillMaxWidth(),
                    placeholder = { Text("Search models...", color = BrainColors.text400) },
                    leadingIcon = { Icon(Icons.Default.Search, null, tint = BrainColors.text400, modifier = Modifier.size(18.dp)) },
                    textStyle = MaterialTheme.typography.bodySmall.copy(color = BrainColors.text100),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = BrainColors.border200,
                        unfocusedBorderColor = BrainColors.border200,
                        focusedContainerColor = BrainColors.bg300,
                        unfocusedContainerColor = BrainColors.bg300
                    ),
                    shape = RoundedCornerShape(10.dp),
                    singleLine = true
                )

                Spacer(modifier = Modifier.height(8.dp))

                if (filteredModels.isEmpty()) {
                    Text(
                        text = "No models found",
                        fontSize = 13.sp,
                        color = BrainColors.text400,
                        modifier = Modifier.padding(16.dp)
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier.heightIn(max = 300.dp)
                    ) {
                        items(filteredModels) { model ->
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(8.dp))
                                    .clickable {
                                        onSelect(model)
                                    }
                                    .background(
                                        if (model == selected) BrainColors.accentMain100.copy(alpha = 0.1f)
                                        else BrainColors.bg300.copy(alpha = 0f)
                                    )
                                    .padding(horizontal = 12.dp, vertical = 10.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    text = model,
                                    fontSize = 14.sp,
                                    color = if (model == selected) BrainColors.accentMain100 else BrainColors.text200,
                                    fontWeight = if (model == selected) FontWeight.Medium else FontWeight.Normal,
                                    modifier = Modifier.weight(1f)
                                )
                                if (model == selected) {
                                    Icon(
                                        Icons.Default.Check,
                                        null,
                                        tint = BrainColors.accentMain100,
                                        modifier = Modifier.size(18.dp)
                                    )
                                }
                            }
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = onDismiss,
                colors = ButtonDefaults.textButtonColors(contentColor = BrainColors.text300)
            ) {
                Text("Close")
            }
        }
    )
}
