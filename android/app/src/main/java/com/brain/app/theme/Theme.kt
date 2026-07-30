package com.brain.app.theme

import android.app.Activity
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

private val BrainDarkScheme = darkColorScheme(
    primary = BrainColors.accentMain100,
    onPrimary = BrainColors.alwaysWhite,
    primaryContainer = BrainColors.accentMain000,
    onPrimaryContainer = BrainColors.text100,
    secondary = BrainColors.accentSecondary100,
    onSecondary = BrainColors.alwaysWhite,
    secondaryContainer = BrainColors.bg300,
    onSecondaryContainer = BrainColors.text200,
    tertiary = BrainColors.accentMain200,
    onTertiary = BrainColors.alwaysWhite,
    background = BrainColors.bg000,
    onBackground = BrainColors.text100,
    surface = BrainColors.bg000,
    onSurface = BrainColors.text100,
    surfaceVariant = BrainColors.bg200,
    onSurfaceVariant = BrainColors.text200,
    outline = BrainColors.border200,
    outlineVariant = BrainColors.border100,
    error = BrainColors.danger100,
    onError = BrainColors.alwaysWhite,
    errorContainer = BrainColors.dangerBg,
    onErrorContainer = BrainColors.danger200,
    inverseSurface = BrainColors.text100,
    inverseOnSurface = BrainColors.bg000,
)

@Composable
fun BrainTheme(content: @Composable () -> Unit) {
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            window.statusBarColor = BrainColors.bg000.toArgb()
            window.navigationBarColor = BrainColors.bg000.toArgb()
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = false
                isAppearanceLightNavigationBars = false
            }
        }
    }

    MaterialTheme(
        colorScheme = BrainDarkScheme,
        shapes = BrainMaterialShapes,
        content = content
    )
}
