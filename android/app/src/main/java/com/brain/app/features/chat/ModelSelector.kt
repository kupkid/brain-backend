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
    selectedModel: String,
    onModelSelected: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var isExpanded by remember { mutableStateOf(false) }
    var searchQuery by remember { mutableStateOf("") }

    val filteredModels = remember(models, searchQuery) {
        if (searchQuery.isBlank()) models
        else models.filter { it.contains(searchQuery, ignoreCase = true) }
    }

    val displayName = selectedModel.ifBlank { "Select model" }

    Box(modifier = modifier) {
        // Trigger button
        Row(
            modifier = Modifier
                .clip(RoundedCornerShape(8.dp))
                .clickable { isExpanded = !isExpanded }
                .padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = displayName,
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
                color = BrainColors.text200,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f, fill = false)
            )
            Spacer(modifier = Modifier.width(4.dp))
            Icon(
                if (isExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = null,
                tint = BrainColors.text400,
                modifier = Modifier.size(16.dp)
            )
        }

        // Dropdown
        AnimatedVisibility(
            visible = isExpanded,
            enter = expandVertically(),
            exit = shrinkVertically()
        ) {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 4.dp),
                colors = CardDefaults.cardColors(containerColor = BrainColors.bg300),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(8.dp)) {
                    // Search
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
                            focusedContainerColor = BrainColors.bg200,
                            unfocusedContainerColor = BrainColors.bg200
                        ),
                        shape = RoundedCornerShape(8.dp),
                        singleLine = true
                    )

                    Spacer(modifier = Modifier.height(4.dp))

                    // Model list
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
                                            onModelSelected(model)
                                            isExpanded = false
                                            searchQuery = ""
                                        }
                                        .background(
                                            if (model == selectedModel) BrainColors.accentMain100.copy(alpha = 0.1f)
                                            else BrainColors.bg200.copy(alpha = 0f)
                                        )
                                        .padding(horizontal = 12.dp, vertical = 10.dp),
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text(
                                        text = model,
                                        fontSize = 14.sp,
                                        color = if (model == selectedModel) BrainColors.accentMain100 else BrainColors.text200,
                                        fontWeight = if (model == selectedModel) FontWeight.Medium else FontWeight.Normal,
                                        modifier = Modifier.weight(1f)
                                    )
                                    if (model == selectedModel) {
                                        Icon(
                                            Icons.Default.Check,
                                            null,
                                            tint = BrainColors.accentSecondary100,
                                            modifier = Modifier.size(18.dp)
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
