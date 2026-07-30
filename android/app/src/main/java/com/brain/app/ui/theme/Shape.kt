package com.brain.app.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.ui.unit.dp

object AppShapes {
    val CardLarge = RoundedCornerShape(28.dp)
    val CardMedium = RoundedCornerShape(24.dp)
    val CardSmall = RoundedCornerShape(16.dp)

    val ButtonPill = RoundedCornerShape(50)
    val ButtonRounded = RoundedCornerShape(20.dp)
    val ButtonSquared = RoundedCornerShape(12.dp)

    val InputField = RoundedCornerShape(20.dp)
    val SearchField = ButtonPill

    val Chip = RoundedCornerShape(12.dp)
    val Tag = RoundedCornerShape(50)

    val Dialog = RoundedCornerShape(28.dp)
    val BottomSheet = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp)

    val Avatar = RoundedCornerShape(50)
    val IconButton = RoundedCornerShape(50)

    val ListItem = RoundedCornerShape(16.dp)
    val ListItemFirst = RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp)
    val ListItemLast = RoundedCornerShape(bottomStart = 16.dp, bottomEnd = 16.dp)

    val CardLargeInner12 = RoundedCornerShape(16.dp)
    val CardLargeInner8 = RoundedCornerShape(20.dp)

    val MessageBubbleInner = RoundedCornerShape(8.dp)

    val MessageOutgoing = RoundedCornerShape(
        topStart = 20.dp, topEnd = 20.dp,
        bottomStart = 20.dp, bottomEnd = 6.dp
    )
    val MessageIncoming = RoundedCornerShape(
        topStart = 20.dp, topEnd = 20.dp,
        bottomStart = 6.dp, bottomEnd = 20.dp
    )
}

val Shapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(28.dp)
)
