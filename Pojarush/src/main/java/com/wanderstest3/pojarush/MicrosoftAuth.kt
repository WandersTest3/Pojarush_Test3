package com.wanderstest3.pojarush

import android.net.Uri
import okhttp3.FormBody
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException

object MicrosoftAuth {
    private const val CLIENT_ID = "00000000402b5328"
    private const val SCOPE = "service::user.auth.xboxlive.com::MBI_SSL"
    const val REDIRECT = "https://login.live.com/oauth20_desktop.srf"
    private const val TOKEN = "https://login.live.com/oauth20_token.srf"
    private val JSON = "application/json".toMediaType()

    val authorizeUrl: String = Uri.parse("https://login.live.com/oauth20_authorize.srf").buildUpon()
        .appendQueryParameter("client_id", CLIENT_ID)
        .appendQueryParameter("response_type", "code")
        .appendQueryParameter("scope", SCOPE)
        .appendQueryParameter("redirect_uri", REDIRECT)
        .appendQueryParameter("prompt", "select_account")
        .build().toString()

    class AuthException(message: String) : IOException(message)

    fun loginWithCode(code: String): Profile {
        val json = postForm(
            TOKEN,
            mapOf(
                "client_id" to CLIENT_ID,
                "code" to code,
                "grant_type" to "authorization_code",
                "redirect_uri" to REDIRECT,
                "scope" to SCOPE,
            )
        )
        return minecraftLogin(json.getString("access_token"), json.optString("refresh_token"))
    }

    fun refresh(profile: Profile): Profile {
        if (!profile.microsoft || profile.refreshToken.isBlank()) return profile
        if (System.currentTimeMillis() < profile.expiresAt - 5 * 60_000L) return profile
        val (status, json) = postFormRaw(
            TOKEN,
            mapOf(
                "client_id" to CLIENT_ID,
                "refresh_token" to profile.refreshToken,
                "grant_type" to "refresh_token",
                "redirect_uri" to REDIRECT,
                "scope" to SCOPE,
            )
        )
        if (status !in 200..299) throw AuthException(json.optString("error_description", "The Microsoft session expired: sign in again"))
        return minecraftLogin(json.getString("access_token"), json.optString("refresh_token", profile.refreshToken))
    }

    private fun minecraftLogin(msaToken: String, refreshToken: String): Profile {
        fun xbox(ticket: String) = postJson(
            "https://user.auth.xboxlive.com/user/authenticate",
            JSONObject()
                .put("Properties", JSONObject().put("AuthMethod", "RPS").put("SiteName", "user.auth.xboxlive.com").put("RpsTicket", ticket))
                .put("RelyingParty", "http://auth.xboxlive.com")
                .put("TokenType", "JWT")
        )
        val xbl = runCatching { xbox(msaToken) }.getOrElse { xbox("d=$msaToken") }
        val uhs = xbl.getJSONObject("DisplayClaims").getJSONArray("xui").getJSONObject(0).getString("uhs")
        val xsts = postJson(
            "https://xsts.auth.xboxlive.com/xsts/authorize",
            JSONObject()
                .put("Properties", JSONObject().put("SandboxId", "RETAIL").put("UserTokens", JSONArray().put(xbl.getString("Token"))))
                .put("RelyingParty", "rp://api.minecraftservices.com/")
                .put("TokenType", "JWT"),
            xstsErrors = true
        )
        val mc = postJson(
            "https://api.minecraftservices.com/authentication/login_with_xbox",
            JSONObject().put("identityToken", "XBL3.0 x=$uhs;${xsts.getString("Token")}")
        )
        val token = mc.getString("access_token")
        val items = getJson("https://api.minecraftservices.com/entitlements/mcstore", token).optJSONArray("items")
        if (items == null || items.length() == 0) throw AuthException("This Microsoft account does not own Minecraft: Java Edition")
        val profile = getJson("https://api.minecraftservices.com/minecraft/profile", token)
        return Profile(
            name = profile.getString("name"),
            microsoft = true,
            uuid = profile.getString("id"),
            accessToken = token,
            refreshToken = refreshToken,
            expiresAt = System.currentTimeMillis() + mc.optLong("expires_in", 86400) * 1000L,
        )
    }

    private fun postFormRaw(url: String, form: Map<String, String>): Pair<Int, JSONObject> {
        val body = FormBody.Builder().apply { form.forEach { (k, v) -> add(k, v) } }.build()
        Http.client.newCall(Request.Builder().url(url).post(body).build()).execute().use { r ->
            val text = r.body.string()
            return r.code to runCatching { JSONObject(text) }.getOrElse { JSONObject().put("error", text.take(200)) }
        }
    }

    private fun postForm(url: String, form: Map<String, String>): JSONObject {
        val (status, json) = postFormRaw(url, form)
        if (status !in 200..299) throw AuthException(json.optString("error_description", json.optString("error", "HTTP $status")))
        return json
    }

    private fun postJson(url: String, body: JSONObject, xstsErrors: Boolean = false): JSONObject {
        val request = Request.Builder().url(url)
            .header("Accept", "application/json")
            .post(body.toString().toRequestBody(JSON))
            .build()
        Http.client.newCall(request).execute().use { r ->
            val text = r.body.string()
            val json = runCatching { JSONObject(text) }.getOrElse { JSONObject() }
            if (r.isSuccessful) return json
            if (xstsErrors) {
                val reason = when (json.optLong("XErr")) {
                    2148916233L -> "This account has no Xbox profile: create one at xbox.com"
                    2148916235L -> "Xbox Live is not available in your country"
                    2148916236L, 2148916237L, 2148916238L -> "This account needs an adult to add it to a Microsoft family"
                    else -> null
                }
                if (reason != null) throw AuthException(reason)
            }
            throw AuthException("HTTP ${r.code} from $url")
        }
    }

    private fun getJson(url: String, token: String): JSONObject {
        val request = Request.Builder().url(url).header("Authorization", "Bearer $token").build()
        Http.client.newCall(request).execute().use { r ->
            if (!r.isSuccessful) throw AuthException("HTTP ${r.code} from $url")
            return JSONObject(r.body.string())
        }
    }
}
