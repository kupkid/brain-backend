package com.brain.app.features.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
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
import com.brain.app.theme.BrainShapes

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Header(
    title: String,
    selectedModel: String,
    onTitleChange: (String) -> Unit,
    onModelClick: () -> Unit,
    onMenuClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    var editingTitle by remember { mutableStateOf(false) }
    var titleText by remember(title) { mutableStateOf(title) }

    TopAppBar(
        title = {
            if (editingTitle) {
                OutlinedTextField(
                    value = titleText,
                    onValueChange = { titleText = it },
                    modifier = Modifier.fillMaxWidth(),
                    textStyle = MaterialTheme.typography.titleMedium.copy(
                        color = BrainColors.text100
                    ),
                    singleLine = true,
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = BrainColors.accentMain100,
                        unfocusedBorderColor = BrainColors.border200
                    )
                )
            } else {
                Text(
                    text = title.ifBlank { "New chat" },
                    color = BrainColors.text100,
                    fontSize = 17.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.clickable {
                        editingTitle = true
                        titleText = title
                    }
                )
            }
        },
        navigationIcon = {
            IconButton(onClick = onMenuClick) {
                Icon(
                    Icons.AutoMirrored.Filled.List,
                    contentDescription = "Menu",
                    tint = BrainColors.text300
                )
            }
        },
        actions = {
            ModelChip(
                model = selectedModel,
                onClick = onModelClick
            )
        },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = BrainColors.bg000,
            titleContentColor = BrainColors.text100
        )
    )
}

@Composable
private fun ModelChip(
    model: String,
    onClick: () -> Unit
) {
    val displayName = model.split("/").lastOrNull()?.take(20) ?: model
    Surface(
        modifier = Modifier
            .padding(end = 8.dp)
            .clip(RoundedCornerShape(BrainShapes.full))
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(BrainShapes.full),
        color = BrainColors.bg300
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Icon(
                Icons.Default.SmartToy,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = BrainColors.accentMain100
            )
            Text(
                text = displayName,
                color = BrainColors.text200,
                fontSize = 12.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
            Icon(
                Icons.Default.ExpandMore,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = BrainColors.text400
            )
        }
    }
}
