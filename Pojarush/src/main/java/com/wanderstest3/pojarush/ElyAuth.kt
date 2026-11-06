package com.wanderstest3.pojarush

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.IOException
import java.util.UUID

object ElyAuth {
    private const val AUTHENTICATE = "https://authserver.ely.by/auth/authenticate"
    private val JSON = "application/json; charset=utf-8".toMediaType()

    fun login(username: String, password: String): Profile {
        val clientToken = UUID.randomUUID().toString().replace("-", "")
        val body = JSONObject()
            .put("username", username.trim())
            .put("password", password)
            .put("clientToken", clientToken)
            .put("requestUser", false)
        val request = Request.Builder().url(AUTHENTICATE)
            .header("Accept", "application/json")
            .post(body.toString().toRequestBody(JSON))
            .build()
        Http.client.newCall(request).execute().use { r ->
            val text = r.body.string()
            val json = runCatching { JSONObject(text) }.getOrElse { JSONObject() }
            if (!r.isSuccessful) {
                val message = json.optString("errorMessage").ifBlank { "Ely.by answered HTTP ${r.code}" }
                throw IOException(message)
            }
            val selected = json.optJSONObject("selectedProfile")
                ?: throw IOException("This Ely.by account has no selected profile")
            return Profile(
                name = selected.getString("name"),
                uuid = selected.getString("id"),
                accessToken = json.getString("accessToken"),
                clientToken = json.optString("clientToken", clientToken),
                ely = true,
            )
        }
    }
}
