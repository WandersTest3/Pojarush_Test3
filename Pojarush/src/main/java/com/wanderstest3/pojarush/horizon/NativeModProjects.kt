package com.wanderstest3.pojarush.horizon

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import com.wanderstest3.pojarush.Paths
import java.io.File
import java.io.IOException
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

object NativeModProjects {
    private const val MIXINS = "0d6b103c70633d8fd9443d8d0acd3aadaacedb06"
    private val ID = Regex("^[a-z][a-z0-9_]{1,31}$")

    val root: File get() = File(Paths.publicRoot, "mod-projects")

    fun validId(id: String): Boolean = ID.matches(id)

    fun list(): List<File> =
        root.listFiles()?.filter { File(it, "horizon-native.json").isFile }?.sortedBy { it.name }.orEmpty()

    private fun rust(text: String): String =
        text.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", " ")

    fun create(id: String, name: String, description: String): File {
        if (!validId(id)) throw IOException("invalid")
        val dir = File(root, id)
        if (dir.exists()) throw IOException("exists")
        val mixins = File(dir, "mixins/$id").apply { mkdirs() }
        File(dir, "horizon-native.json").writeText(
            "{\n  \"game\": \"${HorizonManager.VERSION}\",\n  \"id\": \"$id\",\n  \"mixins\": \"$MIXINS\",\n  \"schema\": 1,\n  \"version\": \"0.1.0\"\n}\n"
        )
        File(mixins, "horizon-mixin.toml").writeText(
            """
            [mod]
            id = "$id"
            version = "0.1.0"
            priority = 100

            [[module]]
            id = "runtime"
            file = "crates/horizon-client/src/game.rs"
            name = "$id"
            source = "runtime.rs"

            [[injection]]
            id = "list"
            kind = "HEAD"
            target = "horizon_client::game::native_hooks::Game<'_>::native_mods"
            helper = "list.rs"
            bindings = [
              { name = "mods", ty = "&mut Vec<crate::game::native_hooks::NativeMod>", value = "&mut *mods" },
            ]

            [[injection]]
            id = "settings"
            kind = "HEAD"
            target = "horizon_client::game::native_hooks::Game<'_>::native_settings"
            helper = "settings.rs"
            bindings = [
              { name = "game", ty = "&mut crate::game::Game<'_>", value = "&mut *self" },
              { name = "mod_id", ty = "&str", value = "mod_id" },
              { name = "buttons", ty = "&mut Vec<String>", value = "&mut *buttons" },
            ]

            [[injection]]
            id = "click"
            kind = "HEAD"
            target = "horizon_client::game::native_hooks::Game<'_>::native_settings_click"
            helper = "click.rs"
            bindings = [
              { name = "game", ty = "&mut crate::game::Game<'_>", value = "&mut *self" },
              { name = "mod_id", ty = "&str", value = "mod_id" },
              { name = "button", ty = "usize", value = "button" },
            ]
            """.trimIndent() + "\n"
        )
        File(mixins, "runtime.rs").writeText(
            """
            use std::sync::atomic::{AtomicU32, Ordering};

            pub const ID: &str = "$id";
            pub const NAME: &str = "${rust(name)}";
            pub const VERSION: &str = "0.1.0";
            pub const DESCRIPTION: &str = "${rust(description)}";

            static CLICKS: AtomicU32 = AtomicU32::new(0);

            pub fn settings() -> Vec<String> {
                vec![format!("Clicks: {}", CLICKS.load(Ordering::Relaxed))]
            }

            pub fn click(button: usize) {
                if button == 0 {
                    CLICKS.fetch_add(1, Ordering::Relaxed);
                }
            }
            """.trimIndent() + "\n"
        )
        File(mixins, "list.rs").writeText(
            """
            fn ${id}_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
                use crate::game::$id as m;
                mods.push(crate::game::native_hooks::NativeMod {
                    id: m::ID.into(),
                    title: format!("{} {}", m::NAME, m::VERSION),
                    description: m::DESCRIPTION.into(),
                    keys: Vec::new(),
                });
            }
            """.trimIndent() + "\n"
        )
        File(mixins, "settings.rs").writeText(
            """
            fn ${id}_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
                let _ = game;
                if mod_id == crate::game::$id::ID {
                    buttons.extend(crate::game::$id::settings());
                }
            }
            """.trimIndent() + "\n"
        )
        File(mixins, "click.rs").writeText(
            """
            fn ${id}_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
                let _ = game;
                if mod_id == crate::game::$id::ID {
                    crate::game::$id::click(button);
                }
            }
            """.trimIndent() + "\n"
        )
        return dir
    }

    fun pack(context: Context, dir: File): String {
        val id = dir.name
        val inbox = File(context.filesDir, "import-inbox").apply { mkdirs() }
        val name = "$id.hnmod"
        val part = File(inbox, "$name.part")
        ZipOutputStream(part.outputStream()).use { zip ->
            dir.walkTopDown().filter { it.isFile && !it.name.startsWith(".") }.sortedBy { it.path }.forEach { f ->
                zip.putNextEntry(ZipEntry(f.relativeTo(dir).invariantSeparatorsPath))
                f.inputStream().use { it.copyTo(zip) }
                zip.closeEntry()
            }
        }
        if (!part.renameTo(File(inbox, name))) {
            part.delete()
            throw IOException("rename")
        }
        return name
    }

    fun toInbox(context: Context, uri: Uri): String {
        var name = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
            if (it.moveToFirst()) it.getString(0) else null
        } ?: uri.lastPathSegment?.substringAfterLast('/') ?: "file"
        name = name.replace(Regex("[\\\\/:*?\"<>|]"), "_")
        val inbox = File(context.filesDir, "import-inbox").apply { mkdirs() }
        val part = File(inbox, "$name.part")
        context.contentResolver.openInputStream(uri)?.use { input -> part.outputStream().use { input.copyTo(it) } }
            ?: throw IOException("Empty file")
        if (!part.renameTo(File(inbox, name))) {
            part.delete()
            throw IOException("rename")
        }
        return name
    }
}
