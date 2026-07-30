package com.brain.app.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.ui.unit.dp

object BrainShapes {
    val xs = RoundedCornerShape(4.dp)
    val sm = RoundedCornerShape(6.dp)
    val md = RoundedCornerShape(8.dp)
    val lg = RoundedCornerShape(12.dp)
    val xl = RoundedCornerShape(16.dp)
    val xxl = RoundedCornerShape(20.dp)
    val full = RoundedCornerShape(9999.dp)

    // OpenCodeUI message bubbles
    val messageOutgoing = RoundedCornerShape(20.dp, 20.dp, 20.dp, 6.dp)
    val messageIncoming = RoundedCornerShape(20.dp, 20.dp, 6.dp, 20.dp)

    // Input field
    val inputField = RoundedCornerShape(20.dp)

    // Cards
    val cardLarge = RoundedCornerShape(28.dp)
    val cardMedium = RoundedCornerShape(24.dp)
    val cardSmall = RoundedCornerShape(16.dp)

    // Buttons
    val buttonPill = RoundedCornerShape(9999.dp)
    val buttonRounded = RoundedCornerShape(20.dp)
}

val BrainMaterialShapes = Shapes(
    extraSmall = BrainShapes.xs,
    small = BrainShapes.sm,
    medium = BrainShapes.md,
    large = BrainShapes.lg,
    extraLarge = BrainShapes.xxl
)
