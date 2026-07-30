package com.brain.app.ui

import androidx.compose.animation.*
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.brain.app.ui.theme.BrainColors

// ═══════════════════════════════════════════════════════════════════════════
// MODEL DATA
// ═══════════════════════════════════════════════════════════════════════════

data class ModelItem(
    val id: String,
    val name: String,
    val provider: String,
    val contextWindow: Int = 0,   // tokens
    val supportsReasoning: Boolean = false,
    val supportsVision: Boolean = false,
    val supportsTools: Boolean = true,
)

// ═══════════════════════════════════════════════════════════════════════════
// MODEL SELECTOR — OpenCodeUI pattern
// ═══════════════════════════════════════════════════════════════════════════

@Composable
fun ModelSelector(
    models: List<ModelItem>,
    selectedModel: String?,
    onModelSelected: (ModelItem) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    var searchQuery by remember { mutableStateOf("") }
    val selected = models.find { it.id == selectedModel }

    Column(modifier) {
        // Trigger button
        Surface(
            modifier = Modifier.clickable { expanded = !expanded },
            shape = RoundedCornerShape(10.dp),
            color = BrainColors.Bg200,
            contentColor = BrainColors.Text100,
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp)
            ) {
                Text(
                    selected?.name?.split("/")?.last() ?: "Выберите модель",
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Medium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false)
                )
                Icon(
                    if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    null,
                    modifier = Modifier.size(16.dp),
                    tint = BrainColors.Text400
                )
            }
        }

        // Dropdown panel
        AnimatedVisibility(visible = expanded) {
            Surface(
                modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
                shape = RoundedCornerShape(14.dp),
                color = BrainColors.Bg200,
                contentColor = BrainColors.Text100,
                shadowElevation = 8.dp,
            ) {
                Column(modifier = Modifier.padding(8.dp)) {
                    // Search bar
                    SearchBar(searchQuery, { searchQuery = it })

                    Spacer(Modifier.height(4.dp))

                    // Grouped model list
                    val filtered = if (searchQuery.isBlank()) models
                    else models.filter {
                        it.name.contains(searchQuery, true) ||
                                it.provider.contains(searchQuery, true)
                    }

                    // Pinned section (models with context > 100k)
                    val pinned = filtered.filter { it.contextWindow > 100_000 }
                    val recent = filtered.filter { it.contextWindow in 1..100_000 }
                    val others = filtered.filter { it.contextWindow == 0 && it !in pinned && it !in recent }

                    LazyColumn(
                        modifier = Modifier.heightIn(max = 320.dp),
                        verticalArrangement = Arrangement.spacedBy(1.dp)
                    ) {
                        if (pinned.isNotEmpty()) {
                            item { SectionHeader("Крупные модели") }
                            items(pinned, key = { it.id }) { model ->
                                ModelItemRow(model, model.id == selectedModel) {
                                    onModelSelected(model); expanded = false
                                }
                            }
                        }
                        if (recent.isNotEmpty()) {
                            item { SectionHeader("Другие модели") }
                            items(recent, key = { it.id }) { model ->
                                ModelItemRow(model, model.id == selectedModel) {
                                    onModelSelected(model); expanded = false
                                }
                            }
                        }
                        if (others.isNotEmpty()) {
                            item { SectionHeader("Модели") }
                            items(others, key = { it.id }) { model ->
                                ModelItemRow(model, model.id == selectedModel) {
                                    onModelSelected(model); expanded = false
                                }
                            }
                        }
                        if (filtered.isEmpty()) {
                            item {
                                Text(
                                    "Модели не найдены",
                                    color = BrainColors.Text400,
                                    fontSize = 13.sp,
                                    modifier = Modifier.padding(16.dp).fillMaxWidth(),
                                    textAlign = androidx.compose.ui.text.style.TextAlign.Center
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SearchBar(query: String, onQueryChange: (String) -> Unit) {
    Surface(
        shape = RoundedCornerShape(10.dp),
        color = BrainColors.Bg300.copy(alpha = 0.4f),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(Icons.Default.Search, null, modifier = Modifier.size(16.dp), tint = BrainColors.Text400)
            Spacer(Modifier.width(8.dp))
            androidx.compose.foundation.text.BasicTextField(
                value = query,
                onValueChange = onQueryChange,
                textStyle = LocalTextStyle.current.copy(color = BrainColors.Text100, fontSize = 13.sp),
                modifier = Modifier.weight(1f),
                singleLine = true,
                decorationBox = { inner ->
                    Box {
                        if (query.isEmpty()) {
                            Text("Поиск моделей...", color = BrainColors.Text400, fontSize = 13.sp)
                        }
                        inner()
                    }
                }
            )
        }
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(
        title.uppercase(),
        fontSize = 10.sp,
        fontWeight = FontWeight.SemiBold,
        color = BrainColors.Text400.copy(alpha = 0.6f),
        letterSpacing = 0.5.sp,
        modifier = Modifier.padding(start = 8.dp, top = 12.dp, bottom = 4.dp)
    )
}

@Composable
private fun ModelItemRow(
    model: ModelItem,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    Surface(
        modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp)).clickable(onClick = onClick),
        color = if (isSelected) BrainColors.AccentMain100.copy(alpha = 0.1f) else Color.Transparent,
        contentColor = if (isSelected) BrainColors.AccentMain100 else BrainColors.Text200,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Model name + capability icons
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    model.name.split("/").last(),
                    fontSize = 13.sp,
                    fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal,
                    color = if (isSelected) BrainColors.AccentMain100 else BrainColors.Text100,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                // Capability icons
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    if (model.supportsReasoning) {
                        Icon(Icons.Default.Lightbulb, "Reasoning", Modifier.size(11.dp),
                            tint = BrainColors.Warning.copy(alpha = 0.7f))
                    }
                    if (model.supportsVision) {
                        Icon(Icons.Default.Visibility, "Vision", Modifier.size(11.dp),
                            tint = BrainColors.Info.copy(alpha = 0.7f))
                    }
                    if (model.supportsTools) {
                        Icon(Icons.Default.Build, "Tools", Modifier.size(11.dp),
                            tint = BrainColors.Text400.copy(alpha = 0.5f))
                    }
                }
            }

            // Provider name
            Text(
                model.provider,
                fontSize = 11.sp,
                color = BrainColors.Text400,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(horizontal = 8.dp)
            )

            // Context window
            if (model.contextWindow > 0) {
                val ctx = if (model.contextWindow >= 1_000_000) "${model.contextWindow / 1_000_000}M"
                else "${model.contextWindow / 1000}k"
                Text(ctx, fontSize = 11.sp, color = BrainColors.Text500)
            }

            // Check mark
            if (isSelected) {
                Spacer(Modifier.width(6.dp))
                Icon(Icons.Default.Check, null, Modifier.size(16.dp), tint = BrainColors.AccentMain100)
            }
        }
    }
}
