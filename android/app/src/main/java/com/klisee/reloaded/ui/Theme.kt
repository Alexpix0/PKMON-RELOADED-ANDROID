package com.klisee.reloaded.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.expressiveLightColorScheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.klisee.reloaded.R

/**
 * Google Sans Flex is the typeface Material 3 Expressive is designed around, and it is most of what
 * separates an expressive app from a plain Material one. It is a variable font, so the weight is
 * picked through a variation setting rather than separate files.
 */
private val SansFlex: FontFamily = FontFamily(
    Font(R.font.google_sans_flex, FontWeight.Normal, variationSettings = FontVariation.Settings(FontVariation.weight(400))),
    Font(R.font.google_sans_flex, FontWeight.Medium, variationSettings = FontVariation.Settings(FontVariation.weight(560))),
    Font(R.font.google_sans_flex, FontWeight.SemiBold, variationSettings = FontVariation.Settings(FontVariation.weight(640))),
    Font(R.font.google_sans_flex, FontWeight.Bold, variationSettings = FontVariation.Settings(FontVariation.weight(760))),
)

/** The expressive corner scale: rounder than baseline Material, which is what gives the app its shape. */
private val ExpressiveShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(16.dp),
    medium = RoundedCornerShape(24.dp),
    large = RoundedCornerShape(32.dp),
    extraLarge = RoundedCornerShape(44.dp),
)

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ReloadedTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val context = LocalContext.current
    val scheme = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        dark -> darkColorScheme()
        else -> expressiveLightColorScheme()
    }

    val typography = remember {
        val base = Typography()
        Typography(
            displayLarge = base.displayLarge.copy(fontFamily = SansFlex),
            displayMedium = base.displayMedium.copy(fontFamily = SansFlex),
            displaySmall = base.displaySmall.copy(fontFamily = SansFlex),
            headlineLarge = base.headlineLarge.copy(fontFamily = SansFlex),
            headlineMedium = base.headlineMedium.copy(fontFamily = SansFlex),
            headlineSmall = base.headlineSmall.copy(fontFamily = SansFlex),
            titleLarge = base.titleLarge.copy(fontFamily = SansFlex),
            titleMedium = base.titleMedium.copy(fontFamily = SansFlex),
            titleSmall = base.titleSmall.copy(fontFamily = SansFlex),
            bodyLarge = base.bodyLarge.copy(fontFamily = SansFlex),
            bodyMedium = base.bodyMedium.copy(fontFamily = SansFlex),
            bodySmall = base.bodySmall.copy(fontFamily = SansFlex),
            labelLarge = base.labelLarge.copy(fontFamily = SansFlex),
            labelMedium = base.labelMedium.copy(fontFamily = SansFlex),
            labelSmall = base.labelSmall.copy(fontFamily = SansFlex),
        )
    }

    MaterialExpressiveTheme(
        colorScheme = scheme,
        motionScheme = MotionScheme.expressive(),
        typography = typography,
        shapes = ExpressiveShapes,
        content = content,
    )
}
