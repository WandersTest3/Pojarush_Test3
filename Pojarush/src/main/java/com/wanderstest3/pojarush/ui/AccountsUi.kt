package com.wanderstest3.pojarush.ui

import android.app.Activity
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.basicMarquee
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.wanderstest3.pojarush.Accounts
import com.wanderstest3.pojarush.ElyAuth
import com.wanderstest3.pojarush.MicrosoftAuth
import com.wanderstest3.pojarush.MicrosoftLoginActivity
import com.wanderstest3.pojarush.Profile
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.SkinData
import com.wanderstest3.pojarush.Skins
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.sin

@Composable
fun AccountsCard() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val profiles by Accounts.profiles.collectAsStateWithLifecycle()
    val active by Accounts.profile.collectAsStateWithLifecycle()
    var name by rememberSaveable { mutableStateOf("") }
    var elyUser by rememberSaveable { mutableStateOf("") }
    var elyPass by remember { mutableStateOf("") }
    var elyOpen by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val login = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        val code = result.data?.getStringExtra(MicrosoftLoginActivity.EXTRA_CODE)
        if (result.resultCode != Activity.RESULT_OK || code == null) {
            error = result.data?.getStringExtra(MicrosoftLoginActivity.EXTRA_ERROR)
            return@rememberLauncherForActivityResult
        }
        busy = true
        scope.launch {
            runCatching { withContext(Dispatchers.IO) { MicrosoftAuth.loginWithCode(code) } }
                .onSuccess { Accounts.save(it) }
                .onFailure { e -> error = e.message }
            busy = false
        }
    }

    PojarushCard {
        CardTitle(stringResource(R.string.accounts_title))
        if (profiles.isEmpty()) Hint(stringResource(R.string.account_none))
        profiles.forEach { p ->
            Row(
                modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp)).clickable { Accounts.select(p.key) },
                verticalAlignment = Alignment.CenterVertically
            ) {
                RadioButton(selected = active?.key == p.key, onClick = { Accounts.select(p.key) })
                Text(p.name, modifier = Modifier.weight(1f).basicMarquee(), maxLines = 1, style = MaterialTheme.typography.bodyMedium)
                Text(
                    stringResource(
                        when {
                            p.microsoft -> R.string.account_type_microsoft
                            p.ely -> R.string.account_type_ely
                            else -> R.string.account_type_offline
                        }
                    ),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.primary
                )
                TextButton(onClick = { Accounts.remove(p.key) }) { Text(stringResource(R.string.account_remove)) }
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it.take(16) },
                modifier = Modifier.weight(1f),
                singleLine = true,
                label = { Text(stringResource(R.string.account_name_hint)) },
                isError = name.isNotEmpty() && !Accounts.isValidOfflineName(name),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done)
            )
            Button(enabled = Accounts.isValidOfflineName(name), onClick = {
                Accounts.setOfflineName(name)
                name = ""
            }) { Text(stringResource(R.string.account_add_offline)) }
        }
        if (name.isNotEmpty() && !Accounts.isValidOfflineName(name)) {
            Text(stringResource(R.string.account_name_invalid), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(enabled = !busy, onClick = {
                error = null
                login.launch(Intent(context, MicrosoftLoginActivity::class.java))
            }) { Text(stringResource(if (busy) R.string.account_signing_in else R.string.account_add_microsoft)) }
            OutlinedButton(onClick = { elyOpen = !elyOpen }) { Text(stringResource(R.string.account_add_ely)) }
        }
        if (elyOpen) {
            OutlinedTextField(
                value = elyUser,
                onValueChange = { elyUser = it },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text(stringResource(R.string.account_ely_user)) },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next)
            )
            OutlinedTextField(
                value = elyPass,
                onValueChange = { elyPass = it },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                label = { Text(stringResource(R.string.account_ely_password)) },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = ImeAction.Done)
            )
            Hint(stringResource(R.string.account_ely_hint))
            Button(enabled = !busy && elyUser.isNotBlank() && elyPass.isNotEmpty(), onClick = {
                error = null
                busy = true
                scope.launch {
                    runCatching { withContext(Dispatchers.IO) { ElyAuth.login(elyUser, elyPass) } }
                        .onSuccess {
                            Accounts.save(it)
                            elyPass = ""
                            elyOpen = false
                        }
                        .onFailure { e -> error = e.message }
                    busy = false
                }
            }) { Text(stringResource(R.string.account_ely_login)) }
        }
        error?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
        Hint(stringResource(R.string.account_ely_note))
    }
    active?.let { SkinCard(it) }
}

@Composable
fun SkinCard(profile: Profile) {
    val context = LocalContext.current
    var version by remember { mutableIntStateOf(0) }
    var data by remember { mutableStateOf<SkinData?>(null) }
    var loading by remember { mutableStateOf(true) }
    LaunchedEffect(profile.key, version) {
        loading = true
        data = runCatching { Skins.load(profile) }.getOrNull()
        loading = false
    }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        runCatching { Skins.importLocal(context, profile, uri) }
            .onSuccess { version++ }
            .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.skin_title, profile.name))
        val current = data
        if (current?.skin != null) {
            SkinViewer(current, Modifier.fillMaxWidth().height(260.dp).clip(RoundedCornerShape(12.dp)).background(Color(0xFF2B1A3A)))
            Hint(stringResource(if (current.cape != null) R.string.skin_hint_cape else R.string.skin_hint))
        } else {
            Hint(stringResource(if (loading) R.string.skin_loading else R.string.skin_none))
        }
        if (!profile.microsoft) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { picker.launch(arrayOf("image/png")) }) { Text(stringResource(R.string.skin_choose)) }
                if (Skins.localFile(profile).isFile) {
                    OutlinedButton(onClick = {
                        Skins.clearLocal(profile)
                        version++
                    }) { Text(stringResource(R.string.skin_remove)) }
                }
            }
            Hint(stringResource(R.string.skin_offline_note))
        } else {
            Hint(stringResource(R.string.skin_microsoft_note))
        }
    }
}

@Composable
fun SkinViewer(data: SkinData, modifier: Modifier = Modifier) {
    var angle by remember { mutableFloatStateOf(25f) }
    var dragging by remember { mutableStateOf(false) }
    LaunchedEffect(dragging) {
        while (!dragging) {
            delay(32)
            angle = (angle + 1.2f) % 360f
        }
    }
    val paint = remember { Paint().apply { isFilterBitmap = false; isAntiAlias = false } }
    Canvas(
        modifier = modifier.pointerInput(Unit) {
            detectDragGestures(
                onDragStart = { dragging = true },
                onDragEnd = { dragging = false },
                onDragCancel = { dragging = false }
            ) { change, drag ->
                change.consume()
                angle = (angle + drag.x * 0.6f) % 360f
            }
        }
    ) {
        val a = angle
        drawIntoCanvas { c -> drawModel(c.nativeCanvas, size.width, size.height, data, a, paint) }
    }
}

private class Texels(bitmap: Bitmap) {
    val w = bitmap.width
    val h = bitmap.height
    val px = IntArray(w * h).also { bitmap.getPixels(it, 0, w, 0, 0, w, h) }
}

private val texelCache = java.util.WeakHashMap<Bitmap, Texels>()

private fun texelsOf(b: Bitmap): Texels = synchronized(texelCache) { texelCache.getOrPut(b) { Texels(b) } }

private const val PPU = 6
private const val HALF_SPAN = 13f
private const val ROWS = 33

private class Raster {
    val width = (HALF_SPAN * 2 * PPU).toInt()
    val height = ROWS * PPU
    val color = IntArray(width * height)
    val depth = FloatArray(width * height)
    val bitmap: Bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
}

private var raster: Raster? = null

private fun drawModel(canvas: android.graphics.Canvas, w: Float, h: Float, data: SkinData, angle: Float, paint: Paint) {
    val skin = data.skin ?: return
    val r = raster ?: Raster().also { raster = it }
    java.util.Arrays.fill(r.color, 0)
    java.util.Arrays.fill(r.depth, Float.NEGATIVE_INFINITY)
    val rad = Math.toRadians(angle.toDouble())
    val cs = cos(rad).toFloat()
    val sn = sin(rad).toFloat()
    val skinTex = texelsOf(skin)
    val full = skin.height == skin.width
    val armW = if (data.slim) 3f else 4f

    fun depthOf(x: Float, z: Float) = -x * sn + z * cs

    fun face(
        tex: Texels, k: Float, ul: Float, vt: Float, ur: Float, vb: Float,
        ax: Float, az: Float, bx: Float, bz: Float, top: Float, bottom: Float, bias: Float,
    ) {
        val xa = (ax * cs + az * sn + HALF_SPAN) * PPU
        val xb = (bx * cs + bz * sn + HALF_SPAN) * PPU
        if (xb - xa < 0.05f) return
        val da = depthOf(ax, az) + bias
        val db = depthOf(bx, bz) + bias
        val c0 = kotlin.math.ceil(xa - 0.5f).toInt().coerceAtLeast(0)
        val c1 = kotlin.math.floor(xb - 0.5f).toInt().coerceAtMost(r.width - 1)
        val yTop = (top + 0.5f) * PPU
        val yBottom = (bottom + 0.5f) * PPU
        val r0 = kotlin.math.ceil(yTop - 0.5f).toInt().coerceAtLeast(0)
        val r1 = kotlin.math.floor(yBottom - 0.5f).toInt().coerceAtMost(r.height - 1)
        val uMin = minOf(ul, ur) * k
        val uMax = maxOf(ul, ur) * k
        val vMin = minOf(vt, vb) * k
        val vMax = maxOf(vt, vb) * k
        for (c in c0..c1) {
            val t = (c + 0.5f - xa) / (xb - xa)
            val d = da + t * (db - da)
            val u = ((ul + t * (ur - ul)) * k).coerceIn(uMin, uMax - 0.001f).toInt().coerceIn(0, tex.w - 1)
            for (row in r0..r1) {
                val i = row * r.width + c
                if (d <= r.depth[i]) continue
                val s = (row + 0.5f - yTop) / (yBottom - yTop)
                val v = ((vt + s * (vb - vt)) * k).coerceIn(vMin, vMax - 0.001f).toInt().coerceIn(0, tex.h - 1)
                val texel = tex.px[v * tex.w + u]
                if (texel ushr 24 == 0) continue
                r.depth[i] = d
                r.color[i] = texel or (0xFF shl 24)
            }
        }
    }

    val k = skin.width / 64f
    fun box(px: Float, pz: Float, top: Float, bw: Float, bh: Float, bd: Float, u: Float, v: Float, inf: Float, bias: Float = 0f) {
        val gw = bw / 2f + inf
        val gd = bd / 2f + inf
        val t = top - inf
        val b = top + bh + inf
        if (cs > 0f) face(skinTex, k, u + bd, v + bd, u + bd + bw, v + bd + bh, px - gw, pz + gd, px + gw, pz + gd, t, b, bias)
        if (cs < 0f) face(skinTex, k, u + 2 * bd + bw, v + bd, u + 2 * bd + 2 * bw, v + bd + bh, px + gw, pz - gd, px - gw, pz - gd, t, b, bias)
        if (sn > 0f) face(skinTex, k, u, v + bd, u + bd, v + bd + bh, px - gw, pz - gd, px - gw, pz + gd, t, b, bias)
        if (sn < 0f) face(skinTex, k, u + bd + bw, v + bd, u + 2 * bd + bw, v + bd + bh, px + gw, pz + gd, px + gw, pz - gd, t, b, bias)
    }

    val rightArmX = -(4f + armW / 2f)
    val leftArmX = 4f + armW / 2f
    box(0f, 0f, 0f, 8f, 8f, 8f, 0f, 0f, 0f)
    box(0f, 0f, 8f, 8f, 12f, 4f, 16f, 16f, 0f)
    box(rightArmX, 0f, 8f, armW, 12f, 4f, 40f, 16f, 0f)
    box(leftArmX, 0f, 8f, armW, 12f, 4f, if (full) 32f else 40f, if (full) 48f else 16f, 0f)
    box(-2f, 0f, 20f, 4f, 12f, 4f, 0f, 16f, 0f)
    box(2f, 0f, 20f, 4f, 12f, 4f, if (full) 16f else 0f, if (full) 48f else 16f, 0f)
    if (full) {
        box(0f, 0f, 0f, 8f, 8f, 8f, 32f, 0f, 0.5f, 0.01f)
        box(0f, 0f, 8f, 8f, 12f, 4f, 16f, 32f, 0.25f, 0.01f)
        box(rightArmX, 0f, 8f, armW, 12f, 4f, 40f, 32f, 0.25f, 0.01f)
        box(leftArmX, 0f, 8f, armW, 12f, 4f, 48f, 48f, 0.25f, 0.01f)
        box(-2f, 0f, 20f, 4f, 12f, 4f, 0f, 32f, 0.25f, 0.01f)
        box(2f, 0f, 20f, 4f, 12f, 4f, 0f, 48f, 0.25f, 0.01f)
    }
    val cape = data.cape
    if (cape != null) {
        val ct = texelsOf(cape)
        val ck = cape.width / 64f
        val back = -3.3f
        val front = back + 1f
        val half = 5f
        val top = 8f
        val bottom = 24f
        if (cs < 0f) face(ct, ck, 1f, 1f, 11f, 17f, half, back, -half, back, top, bottom, 0f)
        if (cs > 0f) face(ct, ck, 12f, 1f, 22f, 17f, -half, front, half, front, top, bottom, 0f)
        if (sn > 0f) face(ct, ck, 11f, 1f, 12f, 17f, -half, back, -half, front, top, bottom, 0f)
        if (sn < 0f) face(ct, ck, 0f, 1f, 1f, 17f, half, front, half, back, top, bottom, 0f)
    }
    r.bitmap.setPixels(r.color, 0, r.width, 0, 0, r.width, r.height)
    val scale = h * 0.88f / (32f * PPU)
    val dw = r.width * scale
    val dh = r.height * scale
    val left = (w - dw) / 2f
    val topPx = (h - 32f * PPU * scale) / 2f - 0.5f * PPU * scale
    paint.isFilterBitmap = false
    canvas.drawBitmap(r.bitmap, null, RectF(left, topPx, left + dw, topPx + dh), paint)
}
