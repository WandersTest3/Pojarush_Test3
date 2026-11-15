package com.wanderstest3.pojarush

import android.content.Context
import android.content.res.Configuration
import java.util.Locale

object Locales {
    val all: List<Pair<String, String>> = listOf(
        "en" to "English",
        "pt-BR" to "Português (Brasil)",
        "pt-PT" to "Português (Portugal)",
        "es" to "Español",
        "fr" to "Français",
        "de" to "Deutsch",
        "it" to "Italiano",
        "ru" to "Русский",
        "uk" to "Українська",
        "pl" to "Polski",
        "tr" to "Türkçe",
        "ar" to "العربية",
        "zh-CN" to "简体中文",
        "zh-TW" to "繁體中文",
        "ja" to "日本語",
        "ko" to "한국어",
        "th" to "ไทย",
        "vi" to "Tiếng Việt",
        "id" to "Bahasa Indonesia",
    )

    fun saved(context: Context): String =
        context.applicationContext.getSharedPreferences("pojarush_controls", Context.MODE_PRIVATE).getString("language", "").orEmpty()

    fun wrap(base: Context): Context {
        val tag = saved(base)
        if (tag.isEmpty()) return base
        val locale = Locale.forLanguageTag(tag)
        Locale.setDefault(locale)
        val config = Configuration(base.resources.configuration)
        config.setLocale(locale)
        config.setLayoutDirection(locale)
        return base.createConfigurationContext(config)
    }
}
