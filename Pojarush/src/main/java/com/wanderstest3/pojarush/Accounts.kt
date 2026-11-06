package com.wanderstest3.pojarush

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject

data class Profile(
    val name: String,
    val microsoft: Boolean = false,
    val uuid: String = "",
    val accessToken: String = "",
    val refreshToken: String = "",
    val expiresAt: Long = 0L,
    val xuid: String = "",
    val ely: Boolean = false,
    val clientToken: String = "",
) {
    val key: String
        get() = when {
            microsoft -> "ms:${uuid.replace("-", "").lowercase()}"
            ely -> "ely:${uuid.replace("-", "").lowercase()}"
            else -> "offline:${name.lowercase()}"
        }
}

object Accounts {
    private const val PREFS = "pojarush_accounts"
    private const val LEGACY = "pojarush_account"
    private lateinit var prefs: SharedPreferences
    private val _profiles = MutableStateFlow<List<Profile>>(emptyList())
    val profiles: StateFlow<List<Profile>> = _profiles.asStateFlow()
    private val _profile = MutableStateFlow<Profile?>(null)
    val profile: StateFlow<Profile?> = _profile.asStateFlow()

    fun init(context: Context) {
        if (::prefs.isInitialized) return
        val app = context.applicationContext
        prefs = app.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        if (!prefs.contains("list")) migrateLegacy(app)
        val list = decode(prefs.getString("list", "[]").orEmpty())
        _profiles.value = list
        val active = prefs.getString("active", null)
        _profile.value = list.firstOrNull { it.key == active } ?: list.firstOrNull()
    }

    private fun migrateLegacy(app: Context) {
        val old = app.getSharedPreferences(LEGACY, Context.MODE_PRIVATE)
        val name = old.getString("name", null) ?: return
        val profile = Profile(
            name = name,
            microsoft = old.getBoolean("microsoft", false),
            uuid = old.getString("uuid", "").orEmpty(),
            accessToken = old.getString("access_token", "").orEmpty(),
            refreshToken = old.getString("refresh_token", "").orEmpty(),
            expiresAt = old.getLong("expires_at", 0L),
            xuid = old.getString("xuid", "").orEmpty(),
        )
        prefs.edit().putString("list", encode(listOf(profile))).putString("active", profile.key).apply()
    }

    private fun encode(list: List<Profile>): String {
        val array = JSONArray()
        list.forEach { p ->
            array.put(
                JSONObject()
                    .put("name", p.name)
                    .put("microsoft", p.microsoft)
                    .put("uuid", p.uuid)
                    .put("access_token", p.accessToken)
                    .put("refresh_token", p.refreshToken)
                    .put("expires_at", p.expiresAt)
                    .put("xuid", p.xuid)
                    .put("ely", p.ely)
                    .put("client_token", p.clientToken)
            )
        }
        return array.toString()
    }

    private fun decode(text: String): List<Profile> = runCatching {
        val array = JSONArray(text)
        (0 until array.length()).map { i ->
            val o = array.getJSONObject(i)
            Profile(
                name = o.getString("name"),
                microsoft = o.optBoolean("microsoft"),
                uuid = o.optString("uuid"),
                accessToken = o.optString("access_token"),
                refreshToken = o.optString("refresh_token"),
                expiresAt = o.optLong("expires_at"),
                xuid = o.optString("xuid"),
                ely = o.optBoolean("ely"),
                clientToken = o.optString("client_token"),
            )
        }
    }.getOrDefault(emptyList())

    private fun commit(list: List<Profile>, active: Profile?) {
        prefs.edit().putString("list", encode(list)).putString("active", active?.key).apply()
        _profiles.value = list
        _profile.value = active
    }

    fun save(profile: Profile) {
        val list = _profiles.value.toMutableList()
        val index = list.indexOfFirst { it.key == profile.key }
        if (index >= 0) list[index] = profile else list += profile
        commit(list, profile)
    }

    fun update(profile: Profile) {
        val list = _profiles.value.toMutableList()
        val index = list.indexOfFirst { it.key == profile.key }
        if (index < 0) return
        list[index] = profile
        val active = _profile.value
        commit(list, if (active?.key == profile.key) profile else active)
    }

    fun select(key: String) {
        val target = _profiles.value.firstOrNull { it.key == key } ?: return
        commit(_profiles.value, target)
    }

    fun remove(key: String) {
        val list = _profiles.value.filterNot { it.key == key }
        val active = _profile.value?.takeIf { it.key != key } ?: list.firstOrNull()
        commit(list, active)
    }

    fun setOfflineName(name: String) = save(Profile(name = name.trim()))

    fun signOut() {
        _profile.value?.let { remove(it.key) }
    }

    fun isValidOfflineName(name: String): Boolean = name.trim().matches(Regex("[A-Za-z0-9_]{3,16}"))
}
