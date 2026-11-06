package com.wanderstest3.pojarush

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.content.Context
import com.wanderstest3.pojarush.horizon.HorizonManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Request
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

data class SkinData(val skin: Bitmap?, val cape: Bitmap?, val slim: Boolean)

object Skins {
    private val defaultNames = listOf("steve", "alex", "ari", "efe", "kai", "makena", "noor", "sunny", "zuri")
    private const val PACK_NAME = "Pojarush Skin.zip"

    fun localFile(profile: Profile): File =
        File(Paths.publicRoot, "skins/" + profile.key.replace(':', '_') + ".png")

    private fun httpsOf(url: String) = if (url.startsWith("http://")) "https://" + url.removePrefix("http://") else url

    private fun fetchBitmap(url: String): Bitmap? = runCatching {
        Http.client.newCall(Request.Builder().url(httpsOf(url)).build()).execute().use { r ->
            if (!r.isSuccessful) return@use null
            val bytes = r.body.bytes()
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inScaled = false })
        }
    }.getOrNull()

    private fun fetchJson(url: String, token: String? = null): JSONObject? = runCatching {
        val builder = Request.Builder().url(url)
        if (token != null) builder.header("Authorization", "Bearer $token")
        Http.client.newCall(builder.build()).execute().use { r ->
            if (!r.isSuccessful) return@use null
            JSONObject(r.body.string())
        }
    }.getOrNull()

    private fun detectSlim(skin: Bitmap): Boolean {
        if (skin.height != skin.width) return false
        val k = skin.width / 64f
        val x = (54 * k).toInt().coerceIn(0, skin.width - 1)
        val y = (20 * k).toInt().coerceIn(0, skin.height - 1)
        return (skin.getPixel(x, y) ushr 24) == 0
    }

    suspend fun load(profile: Profile): SkinData = withContext(Dispatchers.IO) {
        val local = localFile(profile)
        if (local.isFile) {
            val bitmap = runCatching { BitmapFactory.decodeFile(local.absolutePath, BitmapFactory.Options().apply { inScaled = false }) }.getOrNull()
            if (bitmap != null) return@withContext SkinData(bitmap, null, detectSlim(bitmap))
        }
        when {
            profile.microsoft -> {
                val json = fetchJson("https://api.minecraftservices.com/minecraft/profile", profile.accessToken.takeIf { it.isNotBlank() })
                val skins = json?.optJSONArray("skins")
                val capes = json?.optJSONArray("capes")
                var skinUrl: String? = null
                var slim = false
                if (skins != null) for (i in 0 until skins.length()) {
                    val o = skins.getJSONObject(i)
                    if (o.optString("state") == "ACTIVE") {
                        skinUrl = o.optString("url")
                        slim = o.optString("variant").equals("SLIM", true)
                    }
                }
                var capeUrl: String? = null
                if (capes != null) for (i in 0 until capes.length()) {
                    val o = capes.getJSONObject(i)
                    if (o.optString("state") == "ACTIVE") capeUrl = o.optString("url")
                }
                SkinData(skinUrl?.let(::fetchBitmap), capeUrl?.let(::fetchBitmap), slim)
            }
            profile.ely -> {
                val json = fetchJson("https://skinsystem.ely.by/textures/" + Uri.encode(profile.name))
                val skin = json?.optJSONObject("SKIN")
                val cape = json?.optJSONObject("CAPE")
                val skinBitmap = skin?.optString("url")?.takeIf { it.isNotBlank() }?.let(::fetchBitmap)
                    ?: fetchBitmap("https://skinsystem.ely.by/skins/" + Uri.encode(profile.name) + ".png")
                val slim = skin?.optJSONObject("metadata")?.optString("model").equals("slim", true)
                SkinData(skinBitmap, cape?.optString("url")?.takeIf { it.isNotBlank() }?.let(::fetchBitmap), slim)
            }
            else -> SkinData(null, null, false)
        }
    }

    fun importLocal(context: Context, profile: Profile, uri: Uri) {
        val bytes = context.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: throw IOException("Empty file")
        val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
            ?: throw IOException("Not a PNG image")
        val ok = bitmap.width >= 64 && (bitmap.height == bitmap.width || bitmap.height * 2 == bitmap.width)
        if (!ok) throw IOException("A skin is 64x64 or 64x32 pixels (or the same ratio at a higher resolution)")
        val file = localFile(profile)
        file.parentFile?.mkdirs()
        file.writeBytes(bytes)
        buildPack()
    }

    fun clearLocal(profile: Profile) {
        localFile(profile).delete()
        buildPack()
    }

    fun buildPack() {
        val active = Accounts.profile.value
        val pack = File(HorizonManager.resourcePacksDir, PACK_NAME)
        val source = if (active == null || active.microsoft) null else localFile(active).takeIf { it.isFile }
        if (source == null) {
            pack.delete()
            return
        }
        HorizonManager.resourcePacksDir.mkdirs()
        val bytes = source.readBytes()
        ZipOutputStream(pack.outputStream().buffered()).use { zip ->
            fun put(name: String, data: ByteArray) {
                zip.putNextEntry(ZipEntry(name))
                zip.write(data)
                zip.closeEntry()
            }
            put(
                "pack.mcmeta",
                """{"pack":{"pack_format":97,"min_format":97,"max_format":97,"description":"Pojarush offline skin"}}""".toByteArray()
            )
            for (name in defaultNames) {
                put("assets/minecraft/textures/entity/player/wide/$name.png", bytes)
                put("assets/minecraft/textures/entity/player/slim/$name.png", bytes)
            }
        }
    }
}
