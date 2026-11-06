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
                Text(p.name, modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodyMedium)
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

private class Face(val depth: Float, val src: Rect, val dst: RectF, val bitmap: Bitmap, val layer: Float = 0f)

private fun drawModel(canvas: android.graphics.Canvas, w: Float, h: Float, data: SkinData, angle: Float, paint: Paint) {
    val skin = data.skin ?: return
    val rad = Math.toRadians(angle.toDouble())
    val cs = cos(rad).toFloat()
    val sn = sin(rad).toFloat()
    val scale = h * 0.88f / 32f
    val cx = w / 2f
    val top0 = (h - 32f * scale) / 2f
    val k = skin.width / 64f
    val full = skin.height == skin.width
    val armW = if (data.slim) 3f else 4f
    val faces = ArrayList<Face>()

    fun sx(x: Float, z: Float) = cx + (x * cs + z * sn) * scale
    fun src(l: Float, t: Float, r: Float, b: Float) = Rect((l * k).toInt(), (t * k).toInt(), (r * k).toInt(), (b * k).toInt())

    fun add(bitmap: Bitmap, s: Rect, ax: Float, az: Float, bx: Float, bz: Float, top: Float, bottom: Float, layer: Float = 0f) {
        val xa = sx(ax, az)
        val xb = sx(bx, bz)
        if (abs(xb - xa) < 0.5f) return
        val depth = -((ax + bx) / 2f) * sn + ((az + bz) / 2f) * cs
        faces += Face(depth, s, RectF(minOf(xa, xb), top0 + top * scale, maxOf(xa, xb), top0 + bottom * scale), bitmap, layer)
    }

    fun box(bitmap: Bitmap, px: Float, pz: Float, top: Float, bw: Float, bh: Float, bd: Float, u: Float, v: Float, inf: Float, layer: Float = 0f) {
        val gw = bw / 2f + inf
        val gd = bd / 2f + inf
        val t = top - inf
        val b = top + bh + inf
        if (cs > 0f) add(bitmap, src(u + bd, v + bd, u + bd + bw, v + bd + bh), px - gw, pz + gd, px + gw, pz + gd, t, b, layer)
        if (cs < 0f) add(bitmap, src(u + 2 * bd + bw, v + bd, u + 2 * bd + 2 * bw, v + bd + bh), px + gw, pz - gd, px - gw, pz - gd, t, b, layer)
        if (sn > 0f) add(bitmap, src(u, v + bd, u + bd, v + bd + bh), px - gw, pz - gd, px - gw, pz + gd, t, b, layer)
        if (sn < 0f) add(bitmap, src(u + bd + bw, v + bd, u + 2 * bd + bw, v + bd + bh), px + gw, pz + gd, px + gw, pz - gd, t, b, layer)
    }

    val rightArmX = -(4f + armW / 2f)
    val leftArmX = 4f + armW / 2f
    box(skin, 0f, 0f, 0f, 8f, 8f, 8f, 0f, 0f, 0f)
    box(skin, 0f, 0f, 8f, 8f, 12f, 4f, 16f, 16f, 0f)
    box(skin, rightArmX, 0f, 8f, armW, 12f, 4f, 40f, 16f, 0f)
    box(skin, leftArmX, 0f, 8f, armW, 12f, 4f, if (full) 32f else 40f, if (full) 48f else 16f, 0f)
    box(skin, -2f, 0f, 20f, 4f, 12f, 4f, 0f, 16f, 0f)
    box(skin, 2f, 0f, 20f, 4f, 12f, 4f, if (full) 16f else 0f, if (full) 48f else 16f, 0f)
    if (full) {
        box(skin, 0f, 0f, 0f, 8f, 8f, 8f, 32f, 0f, 0.5f, layer = 0.01f)
        box(skin, 0f, 0f, 8f, 8f, 12f, 4f, 16f, 32f, 0.25f, layer = 0.01f)
        box(skin, rightArmX, 0f, 8f, armW, 12f, 4f, 40f, 32f, 0.25f, layer = 0.01f)
        box(skin, leftArmX, 0f, 8f, armW, 12f, 4f, 48f, 48f, 0.25f, layer = 0.01f)
        box(skin, -2f, 0f, 20f, 4f, 12f, 4f, 0f, 32f, 0.25f, layer = 0.01f)
        box(skin, 2f, 0f, 20f, 4f, 12f, 4f, 0f, 48f, 0.25f, layer = 0.01f)
    }
    val cape = data.cape
    if (cape != null) {
        val ck = cape.width / 64f
        fun csrc(l: Float, t: Float, r: Float, b: Float) = Rect((l * ck).toInt(), (t * ck).toInt(), (r * ck).toInt(), (b * ck).toInt())
        val cz = -3.2f
        val half = 5f
        val top = 8f
        val bottom = 24f
        if (cs < 0f) add(cape, csrc(1f, 1f, 11f, 17f), -half, cz, half, cz, top, bottom, layer = -0.02f)
        if (cs > 0f) add(cape, csrc(12f, 1f, 22f, 17f), half, cz, -half, cz, top, bottom, layer = -0.02f)
        if (sn > 0f) add(cape, csrc(11f, 1f, 12f, 17f), half, cz, half, cz + 1f, top, bottom, layer = -0.02f)
        if (sn < 0f) add(cape, csrc(0f, 1f, 1f, 17f), -half, cz, -half, cz + 1f, top, bottom, layer = -0.02f)
    }
    faces.sortWith(compareBy({ it.depth }, { it.layer }))
    faces.forEach { canvas.drawBitmap(it.bitmap, it.src, it.dst, paint) }
}
