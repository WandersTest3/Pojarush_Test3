package com.wanderstest3.pojarush.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.BuildConfig
import com.wanderstest3.pojarush.R
import java.net.HttpURLConnection
import java.net.URL
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

object Updates {
    private const val PREFS = "pojarush_updates"
    private const val SEEN = "changelog_seen"
    private const val DISMISSED = "update_dismissed"
    private const val RELEASES = "https://api.github.com/repos/WandersTest3/Pojarush_Test3/releases/latest"

    data class Release(val version: String, val url: String, val notes: String)

    class Entry(val version: String, val en: List<String>, val pt: List<String>)

    val CHANGELOG = listOf(
        Entry(
            "1.11.29",
            listOf(
                "New Music screen: add several songs and play them shuffled or in order in the launcher menu and in the game.",
                "A single click with a physical mouse is no longer read as two clicks (the inventory double-click bug).",
            ),
            listOf(
                "Nova tela de Músicas: adicione várias músicas e toque em ordem aleatória ou sequencial no menu do launcher e no jogo.",
                "Um clique com mouse físico não é mais lido como dois cliques (bug do duplo clique no inventário).",
            ),
        ),
        Entry(
            "1.11.28",
            listOf(
                "Changelog screen after each update and a notice when a newer release exists on GitHub.",
                "The keyboard now shows suggestions and the corrector in text fields.",
                "Holding the pause button no longer reopens the game.",
                "The game stays paused while a mod's settings are open.",
                "Options > Mods pages scroll fully and show every cape and entity tile.",
                "Fixed the freeze when clicking options such as Graphics sliders and buttons.",
                "Hold time to break blocks at 0 ms no longer clicks items while moving the cursor in the inventory.",
                "Launcher music can now play in the game, with its settings in the floating button menu (Music tab).",
                "Mod API level 16: input.select-hotbar and input.swap-hands (the F key).",
            ),
            listOf(
                "Tela de changelog após cada atualização e aviso quando existe uma versão mais nova no GitHub.",
                "O teclado agora mostra sugestões e o corretor nos campos de texto.",
                "Segurar o botão de pausa não reabre mais o jogo.",
                "O jogo continua pausado enquanto as configurações de um mod estão abertas.",
                "As páginas de Options > Mods rolam por completo e mostram todas as capas e entidades.",
                "Corrigido o travamento ao clicar em opções como sliders e botões de Gráficos.",
                "Tempo para quebrar blocos em 0 ms não clica mais em itens ao mover o cursor no inventário.",
                "A música do launcher agora pode tocar no jogo, com configurações no menu do botão flutuante (aba Música).",
                "API de mods nível 16: input.select-hotbar e input.swap-hands (a tecla F).",
            ),
        ),
        Entry(
            "1.11.27",
            listOf(
                "Horizon's new WebAssembly .hmod mods.",
                "Auto Totem 1.0.2 with anti-ban timing options.",
            ),
            listOf(
                "Novos mods .hmod WebAssembly do Horizon.",
                "Auto Totem 1.0.2 com opções de tempo anti-ban.",
            ),
        ),
        Entry(
            "1.11.26",
            listOf(
                "Held keys survive screens.",
                "Auto Totem 1.0.1 with a HUD counter.",
                "Keystrokes port.",
            ),
            listOf(
                "Teclas seguradas sobrevivem às telas.",
                "Auto Totem 1.0.1 com contador no HUD.",
                "Port do Keystrokes.",
            ),
        ),
        Entry(
            "1.11.25",
            listOf(
                "PR Auto Totem mod.",
                "Mod API level 14: inventory and off hand swap.",
            ),
            listOf(
                "Mod PR Auto Totem.",
                "API de mods nível 14: inventário e troca com a mão secundária.",
            ),
        ),
    )

    fun current(): String = BuildConfig.VERSION_NAME.substringBefore('-')

    fun parse(version: String): List<Int> =
        version.trim().removePrefix("v").removePrefix("V").substringBefore('-').split('.').map { it.toIntOrNull() ?: 0 }

    fun compare(a: String, b: String): Int {
        val x = parse(a)
        val y = parse(b)
        for (i in 0 until maxOf(x.size, y.size)) {
            val d = x.getOrElse(i) { 0 }.compareTo(y.getOrElse(i) { 0 })
            if (d != 0) return d
        }
        return 0
    }

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun unseen(context: Context): List<Entry> {
        val p = prefs(context)
        val seen = p.getString(SEEN, null)
        val now = current()
        if (seen == null) {
            p.edit().putString(SEEN, now).apply()
            return emptyList()
        }
        return CHANGELOG.filter { compare(it.version, seen) > 0 && compare(it.version, now) <= 0 }
    }

    fun markSeen(context: Context) {
        prefs(context).edit().putString(SEEN, current()).apply()
    }

    fun dismissed(context: Context, version: String) {
        prefs(context).edit().putString(DISMISSED, version).apply()
    }

    fun wasDismissed(context: Context, version: String) = prefs(context).getString(DISMISSED, null) == version

    fun fetchLatest(): Release? = try {
        val connection = URL(RELEASES).openConnection() as HttpURLConnection
        connection.connectTimeout = 8000
        connection.readTimeout = 8000
        connection.setRequestProperty("Accept", "application/vnd.github+json")
        connection.setRequestProperty("User-Agent", "Pojarush/${current()}")
        try {
            if (connection.responseCode != 200) {
                null
            } else {
                val json = JSONObject(connection.inputStream.bufferedReader().use { it.readText() })
                if (json.optBoolean("draft") || json.optBoolean("prerelease")) {
                    null
                } else {
                    val tag = json.optString("tag_name")
                    if (tag.isBlank()) null else Release(tag.removePrefix("v"), json.optString("html_url"), json.optString("body"))
                }
            }
        } finally {
            connection.disconnect()
        }
    } catch (_: Exception) {
        null
    }

    fun portuguese(): Boolean = Locale.getDefault().language == "pt"
}

@Composable
fun UpdateDialogs() {
    val context = LocalContext.current
    var changelog by remember { mutableStateOf(Updates.unseen(context)) }
    var release by remember { mutableStateOf<Updates.Release?>(null) }
    LaunchedEffect(Unit) {
        val latest = withContext(Dispatchers.IO) { Updates.fetchLatest() }
        if (latest != null && Updates.compare(latest.version, Updates.current()) > 0 && !Updates.wasDismissed(context, latest.version)) {
            release = latest
        }
    }
    if (changelog.isNotEmpty()) {
        val pt = Updates.portuguese()
        AlertDialog(
            onDismissRequest = {
                Updates.markSeen(context)
                changelog = emptyList()
            },
            title = { Text(stringResource(R.string.changelog_title, Updates.current())) },
            text = {
                Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState())) {
                    changelog.forEach { entry ->
                        Text(entry.version, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                        (if (pt) entry.pt else entry.en).forEach { Text("• $it") }
                        Spacer(Modifier.height(10.dp))
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    Updates.markSeen(context)
                    changelog = emptyList()
                }) { Text(stringResource(R.string.changelog_ok)) }
            },
        )
    } else {
        release?.let { r ->
            AlertDialog(
                onDismissRequest = { release = null },
                title = { Text(stringResource(R.string.update_title, r.version)) },
                text = { Text(stringResource(R.string.update_message, Updates.current(), r.version)) },
                confirmButton = {
                    TextButton(onClick = {
                        val url = r.url.ifBlank { "https://github.com/WandersTest3/Pojarush_Test3/releases/latest" }
                        runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
                        release = null
                    }) { Text(stringResource(R.string.update_now)) }
                },
                dismissButton = {
                    TextButton(onClick = {
                        Updates.dismissed(context, r.version)
                        release = null
                    }) { Text(stringResource(R.string.update_later)) }
                },
            )
        }
    }
}
