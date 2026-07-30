package com.brain.app.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

// ═══════════════════════════════════════════════════════════════════════════
// Forest Green AMOLED — 1:1 from user's CSS tokens
// HSL → hex conversion of user's dark palette
// ═══════════════════════════════════════════════════════════════════════════

object BrainColors {
    // ── Backgrounds ─────────────────────────────────────────────────────
    // bg-000: 0 0% 0%      → absolute black (AMOLED pixel off)
    val Bg000 = Color(0xFF000000)
    // bg-100: 0 0% 0%      → main background
    val Bg100 = Color(0xFF000000)
    // bg-200: 155 18% 3%   → cards/panels (forest at night)
    val Bg200 = Color(0xFF050D07)
    // bg-300: 155 18% 6%   → input fields, active elements
    val Bg300 = Color(0xFF0A1A0E)
    // bg-400: 155 18% 10%  → hover, modals
    val Bg400 = Color(0xFF112617)

    // ── Text ────────────────────────────────────────────────────────────
    // text-000: 0 0% 100%  → pure white
    val Text000 = Color(0xFFFFFFFF)
    // text-100: 140 12% 92% → main text
    val Text100 = Color(0xFFE3EDE5)
    // text-200: 140 10% 74% → secondary text
    val Text200 = Color(0xFFB5C4B8)
    // text-300: 140 8% 58%  → muted text
    val Text300 = Color(0xFF8A998D)
    // text-400: 140 8% 46%  → dim text
    val Text400 = Color(0xFF6B7A6E)
    // text-500: 140 8% 32%  → very dim
    val Text500 = Color(0xFF4A544C)
    // text-600: 140 12% 16% → barely visible
    val Text600 = Color(0xFF283028)

    // ── Accent ──────────────────────────────────────────────────────────
    // accent-brand: 145 35% 52%
    val AccentBrand = Color(0xFF368A5A)
    // accent-main-000: 155 45% 42%
    val AccentMain000 = Color(0xFF1E7A45)
    // accent-main-100: 155 55% 52%  → primary accent
    val AccentMain100 = Color(0xFF26A75C)
    // accent-main-200: 155 65% 64%  → hover/lighter accent
    val AccentMain200 = Color(0xFF3BC972)
    // accent-secondary-100: 130 25% 50% → swamp/bog
    val AccentSecondary = Color(0xFF5A8C5E)

    // ── Semantic ────────────────────────────────────────────────────────
    val Success100 = Color(0xFF2E8B4C)    // 145 35% 55%
    val Success200 = Color(0xFF236839)    // 145 30% 45%
    val SuccessBg = Color(0xFF091409)     // 155 25% 6%

    val Warning100 = Color(0xFFAA8833)    // 40 50% 55%
    val Warning200 = Color(0xFF8C7029)    // 40 45% 45%
    val WarningBg = Color(0xFF151309)     // 45 25% 6%

    val Danger000 = Color(0xFFC43333)    // 355 50% 52%
    val Danger100 = Color(0xFFE04444)    // 355 60% 58%
    val Danger200 = Color(0xFFF06060)    // 355 65% 66%
    val DangerBg = Color(0xFF18090A)     // 355 25% 8%
    val Danger900 = Color(0xFF3D2426)    // 355 20% 18%

    val Info100 = Color(0xFF44A894)      // 175 40% 55%
    val Info200 = Color(0xFF5CC4B0)      // 175 45% 65%
    val InfoBg = Color(0xFF091411)       // 175 25% 8%

    // ── Borders ─────────────────────────────────────────────────────────
    val Border100 = Color(0xFF1A2C1F)    // 155 15% 12%
    val Border200 = Color(0xFF27402C)    // 155 15% 18%
    val Border300 = Color(0xFF37543D)    // 155 15% 25%

    // ── Special ─────────────────────────────────────────────────────────
    val OnColor = Color.White
    val AlwaysBlack = Color(0xFF000000)
    val AlwaysWhite = Color(0xFFFFFFFF)
}

val LocalDarkMode = compositionLocalOf { true }

private val BrainDarkScheme = darkColorScheme(
    primary = BrainColors.AccentMain100,
    onPrimary = BrainColors.OnColor,
    primaryContainer = BrainColors.AccentMain000,
    onPrimaryContainer = BrainColors.Text100,
    secondary = BrainColors.AccentSecondary,
    onSecondary = BrainColors.OnColor,
    secondaryContainer = BrainColors.Bg300,
    onSecondaryContainer = BrainColors.Text200,
    tertiary = BrainColors.AccentMain200,
    onTertiary = BrainColors.OnColor,
    background = BrainColors.Bg000,
    onBackground = BrainColors.Text100,
    surface = BrainColors.Bg000,
    onSurface = BrainColors.Text100,
    surfaceVariant = BrainColors.Bg200,
    onSurfaceVariant = BrainColors.Text300,
    surfaceContainerLow = BrainColors.Bg100,
    surfaceContainer = BrainColors.Bg200,
    surfaceContainerHigh = BrainColors.Bg300,
    surfaceContainerHighest = BrainColors.Bg400,
    error = BrainColors.Danger100,
    onError = BrainColors.OnColor,
    errorContainer = BrainColors.Danger900,
    onErrorContainer = BrainColors.Danger100,
    outline = BrainColors.Border200,
    outlineVariant = BrainColors.Border100,
)

@Composable
fun BrainTheme(content: @Composable () -> Unit) {
    // No dynamic color — force Forest Green AMOLED on all devices
    val colorScheme = BrainDarkScheme

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = false
                isAppearanceLightNavigationBars = false
            }
            window.statusBarColor = BrainColors.Bg000.toArgb()
            @Suppress("DEPRECATION")
            window.navigationBarColor = BrainColors.Bg000.toArgb()
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        shapes = Shapes,
        content = content
    )
}
