package com.wanderstest3.pojarush.horizon

import android.content.Context
import android.content.Intent
import android.system.Os
import android.util.Log
import com.wanderstest3.pojarush.BuildConfig
import com.wanderstest3.pojarush.Http
import com.wanderstest3.pojarush.JavaRuntime
import com.wanderstest3.pojarush.Paths
import com.wanderstest3.pojarush.Profile
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import okhttp3.Request
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.security.MessageDigest
import java.util.concurrent.atomic.AtomicLong

object HorizonManager {
    private const val TAG = "Horizon"
    const val VERSION = "26.3"
    private const val BUNDLE = "horizon"
    private const val LOG_LINES = 300
    private const val PREPARE_STALL_MS = 12 * 60_000L

    sealed interface State {
        data object Idle : State
        data class Running(val step: String, val progress: Float? = null) : State
        data class Failed(val message: String) : State
        data object Done : State
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var job: Job? = null
    @Volatile private var process: Process? = null
    private val http get() = Http.client
    private val downloadClient: okhttp3.OkHttpClient by lazy {
        Http.client.newBuilder()
            .callTimeout(10, java.util.concurrent.TimeUnit.MINUTES)
            .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
            .retryOnConnectionFailure(true)
            .build()
    }

    private val _state = MutableStateFlow<State>(State.Idle)
    val state = _state.asStateFlow()
    private val _log = MutableStateFlow<List<String>>(emptyList())
    val log = _log.asStateFlow()

    val privateRoot: File get() = File(Paths.privateRoot, "horizon")
    val prepareRoot: File get() = File(privateRoot, "prepare")
    val staging: File get() = File(privateRoot, "minecraft")
    private val javaShim: File get() = File(privateRoot, "java")
    val publicRoot: File get() = File(Paths.publicRoot, "horizon")
    val gameDir: File get() = File(publicRoot, "game")
    val homeDir: File get() = File(privateRoot, "home")
    val cacheRoot: File get() = File(privateRoot, "cache")
    val modsDir: File get() = File(gameDir, "horizon-mods")
    val nativeModsDir: File get() = File(gameDir, "horizon-native-mods")
    val resourcePacksDir: File get() = File(gameDir, "resourcepacks")
    val shaderPacksDir: File get() = File(gameDir, "shaderpacks")
    val modsFile: File get() = File(gameDir, "horizon-mods.txt")

    fun ensureFolders() {
        listOf(gameDir, modsDir, nativeModsDir, resourcePacksDir, shaderPacksDir, File(gameDir, "saves"), File(gameDir, "screenshots"), File(homeDir, "logs"))
            .forEach { runCatching { it.mkdirs() } }
    }
    val resources: File get() = File(prepareRoot, ".local/horizon-client/$VERSION")
    val prepareLog: File get() = File(homeDir, "logs/prepare.log")
    val gameLog: File get() = File(homeDir, "logs/latest.log")
    val stdioLog: File get() = File(homeDir, "logs/stdio.log")
    private val soundsMarker: File get() = File(privateRoot, "sounds.ok")

    private val nativeDir: File get() = Paths.nativeDir
    private val gameLibrary: File get() = File(nativeDir, "libhorizon_android.so")
    private val prepareBinary: File get() = File(nativeDir, "libhorizon_prepare.so")
    private val javaLauncher: File get() = File(nativeDir, "libpojajava.so")

    fun isSupported(): Boolean =
        gameLibrary.isFile && prepareBinary.isFile && javaLauncher.isFile

    fun isPrepared(): Boolean = File(resources, "horizon-resources.json").isFile

    fun hasSounds(): Boolean = soundsMarker.isFile && isPrepared()

    fun isBusy(): Boolean = _state.value is State.Running

    private fun appendLog(line: String) {
        _log.update { (it + line).takeLast(LOG_LINES) }
        runCatching {
            prepareLog.parentFile?.mkdirs()
            prepareLog.appendText(line + "\n")
        }
    }

    private fun step(text: String, progress: Float? = null) {
        _state.value = State.Running(text, progress)
    }

    @Volatile private var javaHeapMb = 0

    fun prepare(context: Context, withSounds: Boolean) {
        if (isBusy()) return
        javaHeapMb = com.wanderstest3.pojarush.controls.ControlSettings(context).javaHeapMb
        _log.value = emptyList()
        runCatching { prepareLog.delete() }
        runCatching { File(homeDir, "exit-code").delete() }
        val app = context.applicationContext
        com.wanderstest3.pojarush.KeepAliveService.start(app, com.wanderstest3.pojarush.PrepareKeepAliveService::class.java)
        job = scope.launch {
            runCatching {
                if (!isSupported()) throw IOException("This device cannot run Pojarush_Test3's Horizon libraries (arm64-v8a only).")
                installBundle(context)
                val javaHome = findJava(context)
                downloadGame(withSounds)
                runPrepare(javaHome, listOf("--prepare", "--root", prepareRoot.absolutePath, "--minecraft-dir", staging.absolutePath, "--server-jar", serverJar.absolutePath))
                if (withSounds) {
                    step("Importing sounds")
                    runPrepare(javaHome, listOf("sounds", "--minecraft-dir", staging.absolutePath, "--root", prepareRoot.absolutePath))
                    soundsMarker.writeText(VERSION)
                }
                if (!isPrepared()) throw IOException("horizon-prepare finished without horizon-resources.json")
            }.onSuccess {
                ensureFolders()
                appendLog("Ready: ${resources.absolutePath}")
                _state.value = State.Done
            }.onFailure { e ->
                Log.e(TAG, "Preparation failed", e)
                appendLog("Error: ${e.message}")
                if (e !is kotlinx.coroutines.CancellationException) _state.value = State.Failed(e.message ?: e.javaClass.simpleName)
            }
            com.wanderstest3.pojarush.KeepAliveService.stop(app, com.wanderstest3.pojarush.PrepareKeepAliveService::class.java)
        }
    }

    fun freeSpace() {
        if (isBusy()) return
        staging.deleteRecursively()
        File(prepareRoot, ".local").listFiles()?.forEach { if (it.name != "horizon-client") it.deleteRecursively() }
        File(Paths.cache, "horizon-tmp").deleteRecursively()
    }

    fun shareLogs(context: android.content.Context) {
        val dir = File(context.cacheDir, "shared").apply { mkdirs() }
        val zip = File(dir, "pojarush-logs.zip")
        java.util.zip.ZipOutputStream(zip.outputStream().buffered()).use { out ->
            fun add(name: String, bytes: ByteArray) {
                out.putNextEntry(java.util.zip.ZipEntry(name))
                out.write(bytes)
                out.closeEntry()
            }
            for (file in listOf(prepareLog, gameLog, stdioLog, File(homeDir, "exit-code"))) {
                if (file.isFile) add(file.name, file.readBytes())
            }
            File(homeDir, "logs").listFiles { f -> f.isFile && f.name != "latest.log" && f.name != "stdio.log" && f.name != "prepare.log" }
                .orEmpty().sortedByDescending { it.lastModified() }.take(3)
                .forEach { add("logs/${it.name}", it.readBytes()) }
            File(homeDir, "crash-reports").listFiles { f -> f.isFile }
                .orEmpty().sortedByDescending { it.lastModified() }.take(3)
                .forEach { add("crash-reports/${it.name}", it.readBytes()) }
            runCatching {
                val process = ProcessBuilder("logcat", "-d", "-t", "4000").redirectErrorStream(true).start()
                add("logcat.txt", process.inputStream.readBytes())
            }
        }
        val uri = androidx.core.content.FileProvider.getUriForFile(context, "${context.packageName}.files", zip)
        val send = android.content.Intent(android.content.Intent.ACTION_SEND).apply {
            type = "application/zip"
            putExtra(android.content.Intent.EXTRA_STREAM, uri)
            addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(android.content.Intent.createChooser(send, null).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    fun lastExitCode(): String? = File(homeDir, "exit-code").takeIf { it.isFile }?.readText()?.trim()

    fun readTail(file: File, lines: Int = 400): String =
        runCatching { file.readLines().takeLast(lines).joinToString("\n") }.getOrDefault("")

    fun cancel() {
        process?.let(::killTree)
        job?.cancel()
        _state.value = State.Idle
    }

    private fun pidOf(process: Process): Int? = runCatching {
        if (android.os.Build.VERSION.SDK_INT >= 33) process.pid().toInt()
        else process.javaClass.getDeclaredField("pid").apply { isAccessible = true }.getInt(process)
    }.getOrNull()

    private fun descendants(root: Int): List<Int> {
        val children = HashMap<Int, MutableList<Int>>()
        File("/proc").listFiles()?.forEach { dir ->
            val pid = dir.name.toIntOrNull() ?: return@forEach
            val stat = runCatching { File(dir, "stat").readText() }.getOrNull() ?: return@forEach
            val ppid = stat.substringAfterLast(')').trim().split(' ').getOrNull(1)?.toIntOrNull() ?: return@forEach
            children.getOrPut(ppid) { mutableListOf() }.add(pid)
        }
        val out = mutableListOf<Int>()
        val queue = ArrayDeque(listOf(root))
        while (queue.isNotEmpty()) {
            val next = queue.removeFirst()
            children[next]?.forEach {
                out += it
                queue += it
            }
        }
        return out
    }

    private fun killTree(process: Process) {
        val pid = pidOf(process)
        val tree = pid?.let(::descendants).orEmpty()
        runCatching { process.destroyForcibly() }
        tree.forEach { child -> runCatching { android.system.Os.kill(child, android.system.OsConstants.SIGKILL) } }
    }

    fun syncLogs() {
        runCatching {
            val out = File(publicRoot.parentFile, "logs").apply { mkdirs() }
            File(homeDir, "logs").listFiles()?.forEach { f ->
                val target = File(out, f.name)
                if (f.isFile && (!target.isFile || target.lastModified() < f.lastModified() || target.length() != f.length())) {
                    f.copyTo(target, overwrite = true)
                    target.setLastModified(f.lastModified())
                }
            }
            val crashes = File(homeDir, "crash-reports")
            if (crashes.isDirectory) {
                val dst = File(publicRoot.parentFile, "crash-reports").apply { mkdirs() }
                crashes.listFiles()?.forEach { f ->
                    val target = File(dst, f.name)
                    if (f.isFile && !target.isFile) f.copyTo(target)
                }
            }
        }
    }

    fun lastCrashReason(): String? = runCatching {
        val report = File(homeDir, "crash-reports").listFiles { f -> f.isFile }?.maxByOrNull { it.lastModified() }
        val exit = File(homeDir, "exit-code")
        if (report == null || !exit.isFile || report.lastModified() + 60_000 < exit.lastModified()) return@runCatching null
        val lines = report.readLines()
        val start = lines.indexOfFirst { it.startsWith("Description:") }
        lines.drop(start + 1).firstOrNull { it.isNotBlank() }?.trim()
    }.getOrNull()

    val LOD_STEPS = listOf(0, 32, 64, 128, 256, 512)

    private fun graphicsLines(): MutableList<String> {
        val file = File(gameDir, "horizon-graphics.txt")
        return if (file.isFile) file.readLines().toMutableList() else mutableListOf()
    }

    private fun setGraphics(key: String, value: String) {
        runCatching {
            val lines = graphicsLines()
            val index = lines.indexOfFirst { it.startsWith("$key:") }
            if (index >= 0) lines[index] = "$key:$value" else lines += "$key:$value"
            gameDir.mkdirs()
            File(gameDir, "horizon-graphics.txt").writeText(lines.joinToString("\n", postfix = "\n"))
        }
    }

    fun lodDistance(): Int = runCatching {
        graphicsLines().firstOrNull { it.startsWith("lodDistance:") }?.substringAfter(':')?.trim()?.toIntOrNull() ?: 0
    }.getOrDefault(0)

    fun setLodDistance(chunks: Int) = setGraphics("lodDistance", chunks.toString())

    fun disableBuiltInCounter() {
        runCatching {
            val file = File(gameDir, "horizon-graphics.txt")
            val lines = if (file.isFile) file.readLines().toMutableList() else mutableListOf()
            val index = lines.indexOfFirst { it.startsWith("overlay:") }
            val current = lines.getOrNull(index)?.substringAfter(':')
            if (current != "counter") return
            lines[index] = "overlay:off"
            gameDir.mkdirs()
            file.writeText(lines.joinToString("\n", postfix = "\n"))
        }
    }

    private fun installBundle(context: Context) {
        step("Installing Horizon's preparation files")
        val marker = File(prepareRoot, ".bundle-version")
        val version = "${BuildConfig.VERSION_CODE}-${BuildConfig.VERSION_NAME}"
        if (marker.isFile && marker.readText() == version) return
        listOf("exporters", "mods", "crates").forEach { File(prepareRoot, it).deleteRecursively() }
        fun copy(path: String) {
            val children = context.assets.list(path).orEmpty()
            if (children.isEmpty()) {
                val target = File(prepareRoot, path.removePrefix("$BUNDLE/"))
                target.parentFile?.mkdirs()
                context.assets.open(path).use { input -> target.outputStream().use { input.copyTo(it) } }
            } else {
                children.forEach { copy("$path/$it") }
            }
        }
        copy(BUNDLE)
        marker.writeText(version)
    }

    private fun findJava(context: Context): File {
        step("Looking for Java ${JavaRuntime.MAJOR}")
        val home = JavaRuntime.ensure(context) { step(it) }
        val bin = File(javaShim, "bin")
        bin.mkdirs()
        val link = File(bin, "java")
        if (link.exists() || runCatching { Os.lstat(link.absolutePath) }.isSuccess) link.delete()
        Os.symlink(javaLauncher.absolutePath, link.absolutePath)
        appendLog("Java: ${home.absolutePath}")
        return home
    }

    private fun lock(): JSONObject = JSONObject(File(prepareRoot, "research-lock.json").readText()).getJSONObject("minecraft")

    private val serverJar: File get() = File(staging, "server-$VERSION.jar")

    private suspend fun downloadGame(withSounds: Boolean) {
        step("Downloading Minecraft $VERSION's version file")
        val lock = lock()
        val versionDir = File(staging, "versions/$VERSION")
        val manifestFile = File(versionDir, "$VERSION.json")
        val accepted = buildList {
            add(lock.getString("manifest_sha1"))
            lock.optJSONArray("accepted_manifest_sha1")?.let { a -> for (i in 0 until a.length()) add(a.getString(i)) }
        }
        if (!(manifestFile.isFile && sha1(manifestFile) in accepted)) {
            var fetched = false
            for (sha in accepted) {
                if (runCatching { download("https://piston-meta.mojang.com/v1/packages/$sha/$VERSION.json", manifestFile, sha) }.isSuccess) {
                    fetched = true
                    break
                }
            }
            if (!fetched) throw IOException("Mojang's $VERSION version file could not be downloaded")
        }
        val manifest = JSONObject(manifestFile.readText())
        val downloads = manifest.getJSONObject("downloads")
        val jobs = mutableListOf<Fetch>()
        downloads.getJSONObject("client").let { jobs += Fetch(it.getString("url"), File(versionDir, "$VERSION.jar"), it.getString("sha1"), it.getLong("size")) }
        downloads.getJSONObject("server").let { jobs += Fetch(it.getString("url"), serverJar, it.getString("sha1"), it.getLong("size")) }
        val libraries = manifest.getJSONArray("libraries")
        for (i in 0 until libraries.length()) {
            val artifact = libraries.getJSONObject(i).optJSONObject("downloads")?.optJSONObject("artifact") ?: continue
            val path = artifact.getString("path")
            jobs += Fetch(artifact.getString("url"), File(staging, "libraries/$path"), artifact.getString("sha1"), artifact.getLong("size"))
        }
        val assetIndex = manifest.getJSONObject("assetIndex")
        val indexFile = File(staging, "assets/indexes/${assetIndex.getString("id")}.json")
        fetchAll(listOf(Fetch(assetIndex.getString("url"), indexFile, assetIndex.getString("sha1"), assetIndex.getLong("size"))), "the asset index")
        val objects = JSONObject(indexFile.readText()).getJSONObject("objects")
        for (key in objects.keys()) {
            if (!withSounds && key.startsWith("minecraft/sounds/")) continue
            val obj = objects.getJSONObject(key)
            val hash = obj.getString("hash")
            val relative = "assets/objects/${hash.take(2)}/$hash"
            jobs += Fetch("https://resources.download.minecraft.net/${hash.take(2)}/$hash", File(staging, relative), hash, obj.getLong("size"))
        }
        fetchAll(jobs, "Minecraft $VERSION")
    }

    private data class Fetch(val url: String, val file: File, val sha1: String, val size: Long)

    private suspend fun fetchAll(jobs: List<Fetch>, what: String) = coroutineScope {
        val total = jobs.sumOf { it.size }.coerceAtLeast(1)
        val done = AtomicLong()
        val gate = Semaphore(8)
        step("Downloading $what", 0f)
        jobs.map { fetch ->
            async {
                gate.withPermit {
                    ensureActive()
                    if (!(fetch.file.isFile && fetch.file.length() == fetch.size && sha1(fetch.file) == fetch.sha1)) {
                        download(fetch.url, fetch.file, fetch.sha1)
                    }
                    val now = done.addAndGet(fetch.size)
                    step("Downloading $what (${now / 1_048_576} / ${total / 1_048_576} MB)", now.toFloat() / total)
                }
            }
        }.awaitAll()
        appendLog("Downloaded $what: ${jobs.size} files")
    }

    private fun download(url: String, target: File, sha1: String) {
        target.parentFile?.mkdirs()
        val temp = File(target.parentFile, target.name + ".part")
        var last: Exception? = null
        repeat(3) {
            try {
                downloadClient.newCall(Request.Builder().url(url).build()).execute().use { response ->
                    if (!response.isSuccessful) throw IOException("HTTP ${response.code} for $url")
                    response.body.byteStream().use { input -> temp.outputStream().use { input.copyTo(it) } }
                }
                val got = sha1(temp)
                if (got != sha1) throw IOException("Checksum mismatch for $url ($got)")
                if (!temp.renameTo(target)) {
                    temp.copyTo(target, overwrite = true)
                    temp.delete()
                }
                return
            } catch (e: Exception) {
                last = e
                temp.delete()
            }
        }
        throw last ?: IOException("Download failed: $url")
    }

    private fun sha1(file: File): String {
        val digest = MessageDigest.getInstance("SHA-1")
        file.inputStream().use { input ->
            val buffer = ByteArray(1 shl 16)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }

    private fun runPrepare(javaHome: File, args: List<String>) {
        step(if (args.first() == "sounds") "Importing sounds" else "Preparing Horizon's resources (Java exporters, a few minutes)")
        appendLog("$ horizon-prepare ${args.joinToString(" ")}")
        val tmp = File(Paths.cache, "horizon-tmp").apply { mkdirs() }
        val process = ProcessBuilder(listOf(prepareBinary.absolutePath) + args)
            .directory(prepareRoot)
            .redirectErrorStream(true)
            .apply {
                environment().apply {
                    put("JAVA_HOME", javaShim.absolutePath)
                    put("POJARUSH_JAVA_HOME", javaHome.absolutePath)
                    put("PATH", "${javaShim.absolutePath}/bin:" + (get("PATH") ?: "/system/bin"))
                    put("HOME", privateRoot.absolutePath)
                    put("TMPDIR", tmp.absolutePath)
                    val heap = javaHeapMb.takeIf { it > 0 }
                    put("_JAVA_OPTIONS", "-Djava.io.tmpdir=${tmp.absolutePath} -Djava.awt.headless=true" + (heap?.let { " -Xmx${it}M" } ?: ""))
                    heap?.let { put("HORIZON_PREPARE_HEAP", "${it}M") }
                    remove("LD_LIBRARY_PATH")
                }
            }
            .start()
        this.process = process
        val lastOutput = java.util.concurrent.atomic.AtomicLong(System.currentTimeMillis())
        val stalled = java.util.concurrent.atomic.AtomicBoolean(false)
        val watchdog = scope.launch {
            while (true) {
                kotlinx.coroutines.delay(15_000)
                val idle = System.currentTimeMillis() - lastOutput.get()
                if (idle > PREPARE_STALL_MS) {
                    stalled.set(true)
                    appendLog("No output from horizon-prepare for ${idle / 60_000} minutes; stopping it.")
                    killTree(process)
                    break
                }
            }
        }
        process.inputStream.bufferedReader().useLines { lines ->
            lines.forEach { line ->
                lastOutput.set(System.currentTimeMillis())
                appendLog(line)
                if (line.startsWith("Exporting ") || line.startsWith("Running ") || line.startsWith("Copying ") || line.startsWith("Writing ") || line.startsWith("Compiling ") || line.startsWith("Importing ")) {
                    step(line)
                }
            }
        }
        val code = process.waitFor()
        watchdog.cancel()
        this.process = null
        if (stalled.get()) throw IOException("horizon-prepare stopped answering (low memory?). Lower the Java memory limit in Graphics or free RAM, then Prepare again; see ${prepareLog.absolutePath}")
        if (code != 0) throw IOException("horizon-prepare stopped with exit code $code; see ${prepareLog.absolutePath}")
    }

    fun stopRunningGame(context: Context) {
        val am = context.getSystemService(Context.ACTIVITY_SERVICE) as android.app.ActivityManager
        val name = context.packageName + ":horizon"
        val pids = am.runningAppProcesses.orEmpty().filter { it.processName == name }.map { it.pid }
        if (pids.isEmpty()) return
        pids.forEach { android.os.Process.killProcess(it) }
        val deadline = android.os.SystemClock.uptimeMillis() + 3000
        while (android.os.SystemClock.uptimeMillis() < deadline &&
            am.runningAppProcesses.orEmpty().any { it.processName == name }) {
            Thread.sleep(100)
        }
    }

    private val prewarmKeyFile get() = File(homeDir, "prewarm-key")
    private val revealFile get() = File(homeDir, "reveal")
    private var lastPrewarm = 0L

    fun isGameRunning(context: Context): Boolean {
        val am = context.getSystemService(Context.ACTIVITY_SERVICE) as android.app.ActivityManager
        val name = context.packageName + ":horizon"
        return am.runningAppProcesses.orEmpty().any { it.processName == name }
    }

    private fun keyOf(intent: Intent) = intent.getStringArrayListExtra(HorizonActivity.EXTRA_ARGS).orEmpty().joinToString("\u001f")

    fun prewarm(context: Context, profile: Profile?, force: Boolean = false) {
        if (!isSupported() || !isPrepared()) return
        val now = android.os.SystemClock.uptimeMillis()
        if (!force && now - lastPrewarm < 700) return
        if (isGameRunning(context)) return
        if (!force && lastRunFailed()) return
        lastPrewarm = now
        runCatching {
            val intent = launchIntent(context, profile).putExtra(HorizonActivity.EXTRA_PREWARM, true)
            revealFile.delete()
            prewarmKeyFile.writeText(keyOf(intent))
            context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NO_ANIMATION))
        }
    }

    private val restartHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private var restartTask: Runnable? = null

    fun restartPrewarm(context: Context, profile: Profile?) {
        val app = context.applicationContext
        restartTask?.let(restartHandler::removeCallbacks)
        val task = Runnable {
            Thread {
                if (isGameRunning(app)) stopRunningGame(app)
                lastPrewarm = 0L
                restartHandler.post { prewarm(app, profile, force = true) }
            }.start()
        }
        restartTask = task
        restartHandler.postDelayed(task, 1200)
    }

    private fun lastRunFailed(): Boolean = runCatching {
        File(homeDir, "exit-code").takeIf { it.isFile }?.readText()?.trim()?.let { it.isNotEmpty() && it != "0" } ?: false
    }.getOrDefault(false)

    fun play(context: Context, profile: Profile?) {
        val intent = launchIntent(context, profile)
        val warm = isGameRunning(context) && runCatching { prewarmKeyFile.readText() == keyOf(intent) }.getOrDefault(false)
        if (warm) {
            revealFile.writeText("1")
            context.startActivity(
                Intent(context, HorizonActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT or Intent.FLAG_ACTIVITY_NO_ANIMATION)
            )
        } else {
            stopRunningGame(context)
            runCatching { prewarmKeyFile.delete() }
            context.startActivity(intent)
        }
    }

    fun consumeReveal(): Boolean = revealFile.exists().also { if (it) revealFile.delete() }

    fun removeLegacyBundledMods(context: Context) {
        val prefs = context.getSharedPreferences("bundled_mods", Context.MODE_PRIVATE)
        val key = "installed_pojarush_zoom.hmod"
        if (!prefs.getBoolean(key, false)) return
        runCatching { File(modsDir, "pojarush_zoom.hmod").delete() }
        prefs.edit().remove(key).apply()
    }

    fun launchIntent(context: Context, profile: Profile?): Intent {
        ensureFolders()
        removeLegacyBundledMods(context)
        disableBuiltInCounter()
        privateRoot.mkdirs()
        val args = arrayListOf(
            "--resources", resources.absolutePath,
            "--game-dir", gameDir.absolutePath,
            "--home", homeDir.absolutePath,
        )
        profile?.name?.takeIf { it.isNotBlank() }?.let { args += listOf("--name", it) }
        val session = File(privateRoot, "session.json")
        session.delete()
        if (profile != null && profile.microsoft && profile.accessToken.isNotBlank()) {
            val json = JSONObject()
                .put("name", profile.name)
                .put("uuid", profile.uuid.replace("-", ""))
                .put("access_token", profile.accessToken)
                .put("user_type", "msa")
            if (profile.xuid.isNotBlank()) json.put("xuid", profile.xuid)
            session.writeText(json.toString())
            runCatching { Os.chmod(session.absolutePath, "600".toInt(8)) }
            args += listOf("--session-file", session.absolutePath)
        }
        return Intent(context, HorizonActivity::class.java)
            .putStringArrayListExtra(HorizonActivity.EXTRA_ARGS, args)
            .putExtra(HorizonActivity.EXTRA_GAME_DIR, gameDir.absolutePath)
            .putExtra(HorizonActivity.EXTRA_HOME, homeDir.absolutePath)
            .putExtra(HorizonActivity.EXTRA_CACHE, cacheRoot.apply { mkdirs() }.absolutePath)
    }
}
