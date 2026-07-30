package com.brain.app.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

val LocalDarkMode = compositionLocalOf { true }

private val AMOLED_BLACK = Color(0xFF000000)

private val DarkColorScheme = darkColorScheme(
    primary = Color(0xFF90CAF9),
    secondary = Color(0xFFCE93D8),
    tertiary = Color(0xFF80CBC4),
    background = AMOLED_BLACK,
    surface = AMOLED_BLACK,
    surfaceVariant = Color(0xFF141414),
    surfaceContainerLow = Color(0xFF0E0E0E),
    surfaceContainer = Color(0xFF161616),
    surfaceContainerHigh = Color(0xFF1E1E1E),
    surfaceContainerHighest = Color(0xFF282828),
    onPrimary = Color.Black,
    onSecondary = Color.Black,
    onBackground = Color.White,
    onSurface = Color.White,
    onSurfaceVariant = Color(0xFFB0B0B0),
    error = Color(0xFFEF5350),
    onError = Color.White,
    outline = Color(0xFF333333),
    outlineVariant = Color(0xFF222222),
)

@Composable
fun BrainTheme(content: @Composable () -> Unit) {
    val colorScheme = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        val ctx = LocalContext.current
        dynamicDarkColorScheme(ctx).copy(
            background = AMOLED_BLACK,
            surface = AMOLED_BLACK,
        )
    } else {
        DarkColorScheme
    }

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = false
                isAppearanceLightNavigationBars = false
            }
            window.statusBarColor = AMOLED_BLACK.toArgb()
            @Suppress("DEPRECATION")
            window.navigationBarColor = AMOLED_BLACK.toArgb()
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        shapes = Shapes,
        content = content
    )
}
