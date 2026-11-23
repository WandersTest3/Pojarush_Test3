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
            "1.11.36",
            listOf(
                "Fixed the game not starting with content mods installed (\"table state identities differ\"): blocks whose template has no state properties now get their own properties in the block registry.",
            ),
            listOf(
                "Corrigido o jogo não iniciar com mods de conteúdo instalados (\"table state identities differ\"): blocos cujo modelo base não tem propriedades de estado agora recebem as próprias propriedades no registro de blocos.",
            ),
        ),
        Entry(
            "1.11.35",
            listOf(
                "Resource packs no longer fail to load as a whole when one block's model states disagree with the block registry (content-mod blocks): that block gets no model and the log names it, and every other pack model loads.",
            ),
            listOf(
                "As resource packs não falham mais por inteiro quando os estados de modelo de um bloco divergem do registro de blocos (blocos de mods de conteúdo): esse bloco fica sem modelo e o log o nomeia, e todos os outros modelos do pack carregam.",
            ),
        ),
        Entry(
            "1.11.34",
            listOf(
                "Fixed the game closing right after opening (\"world resources joined before entry\"): wasm mods now start only after the world resources have loaded.",
            ),
            listOf(
                "Corrigido o jogo fechando logo ao abrir (\"world resources joined before entry\"): os mods wasm agora só iniciam depois que os recursos do mundo carregam.",
            ),
        ),
        Entry(
            "1.11.33",
            listOf(
                "Shader packs without block.properties (Chocapic13 and other old packs) now get the default block IDs, so water, ice, leaves, plants and glowing blocks are recognised: water reflections, water out of the shadow map and waving plants.",
                "Content mods' block data is rebuilt after every game update, which fixes the resource pack reload error that stayed after 1.11.32.",
            ),
            listOf(
                "Shaders sem block.properties (Chocapic13 e outros antigos) agora recebem os IDs de bloco padrão, então água, gelo, folhas, plantas e blocos que brilham são reconhecidos: reflexos na água, água fora do mapa de sombras e plantas balançando.",
                "Os dados de blocos dos mods de conteúdo são refeitos a cada atualização do jogo, o que corrige o erro de recarga de resource packs que continuou na 1.11.32.",
            ),
        ),
        Entry(
            "1.11.32",
            listOf(
                "Fixed the resource reload errors (\"model state definitions disagree\" and \"table registry size mismatch\") with content mods that add blocks with state properties, such as Simply Light.",
                "Mod Menu: icons are matched to their mod by id, wasm mods read icon.png, and mods without an icon get a default one.",
                "The red \"Stopped: Starting...\" line of wasm mods goes away once they are running.",
                "Fresh Animations and other OptiFine models: part names are matched in general (OptiFine's names, case and word order), not one mob at a time.",
                "Shader packs: the log now lists the textures a pass reads blank, to help with shadow and reflection bugs.",
            ),
            listOf(
                "Corrigidos os erros ao recarregar resource packs (\"model state definitions disagree\" e \"table registry size mismatch\") com mods de conteúdo que adicionam blocos com propriedades de estado, como o Simply Light.",
                "Mod Menu: os ícones são ligados ao mod pelo id, mods wasm leem o icon.png e mods sem ícone ganham um padrão.",
                "A linha vermelha \"Stopped: Starting...\" dos mods wasm some quando eles estão rodando.",
                "Fresh Animations e outros modelos OptiFine: os nomes das partes casam de forma geral (nomes do OptiFine, maiúsculas e ordem das palavras), não mob por mob.",
                "Shaders: o log agora lista as texturas que uma passada lê em branco, para ajudar nos bugs de sombra e reflexo.",
            ),
        ),
        Entry(
            "1.11.31",
            listOf(
                "Content .hmod mods with Horizon's current block schema load (collision_shapes, tuple shapes, state_light, own state properties), such as Simply Light.",
                "Horizon wasm .hmod mods: GUI layers, client tick, crosshair target, block states, items, entities and the local player, such as Jade's overlay.",
                "Compiled wasm mods are cached in the app's private folder, so only the first start compiles them.",
                "Fresh Animations: sheep wool and the witch's hat and robe follow the animated model.",
                "Shader packs: a discard is deferred to the end of the fragment shader, avoiding the Adreno driver crash (Solas' water).",
            ),
            listOf(
                "Mods .hmod de conteúdo com o schema atual de blocos do Horizon carregam (collision_shapes, shapes por tupla, state_light, propriedades próprias), como o Simply Light.",
                "Mods .hmod wasm do Horizon: camadas de GUI, tick do cliente, alvo da mira, estados de bloco, itens, entidades e o jogador local, como o overlay do Jade.",
                "Os mods wasm compilados ficam em cache na pasta privada do app, então só a primeira abertura compila.",
                "Fresh Animations: a lã da ovelha e o chapéu e a roupa da bruxa seguem o modelo animado.",
                "Shaders: o discard é adiado para o fim do fragment shader, evitando o crash do driver Adreno (água do Solas).",
            ),
        ),
        Entry(
            "1.11.30",
            listOf(
                "Fresh Animations: layers drawn on a custom body, such as the iron golem's cracks, now animate with it.",
            ),
            listOf(
                "Fresh Animations: camadas desenhadas sobre um corpo customizado, como as rachaduras do golem de ferro, agora animam junto com ele.",
            ),
        ),
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
