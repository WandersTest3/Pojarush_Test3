package com.wanderstest3.pojarush

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

data class Profile(
    val name: String,
    val microsoft: Boolean = false,
    val uuid: String = "",
    val accessToken: String = "",
    val refreshToken: String = "",
    val expiresAt: Long = 0L,
    val xuid: String = "",
)

object Accounts {
    private const val PREFS = "pojarush_account"
    private lateinit var prefs: SharedPreferences
    private val _profile = MutableStateFlow<Profile?>(null)
    val profile = _profile.asStateFlow()

    fun init(context: Context) {
        if (::prefs.isInitialized) return
        prefs = context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        _profile.value = prefs.getString("name", null)?.let {
            Profile(
                name = it,
                microsoft = prefs.getBoolean("microsoft", false),
                uuid = prefs.getString("uuid", "").orEmpty(),
                accessToken = prefs.getString("access_token", "").orEmpty(),
                refreshToken = prefs.getString("refresh_token", "").orEmpty(),
                expiresAt = prefs.getLong("expires_at", 0L),
                xuid = prefs.getString("xuid", "").orEmpty(),
            )
        }
    }

    fun save(profile: Profile) {
        prefs.edit()
            .putString("name", profile.name)
            .putBoolean("microsoft", profile.microsoft)
            .putString("uuid", profile.uuid)
            .putString("access_token", profile.accessToken)
            .putString("refresh_token", profile.refreshToken)
            .putLong("expires_at", profile.expiresAt)
            .putString("xuid", profile.xuid)
            .apply()
        _profile.value = profile
    }

    fun setOfflineName(name: String) = save(Profile(name = name.trim()))

    fun signOut() {
        prefs.edit().clear().apply()
        _profile.value = null
    }

    fun isValidOfflineName(name: String): Boolean = name.trim().matches(Regex("[A-Za-z0-9_]{3,16}"))
}
