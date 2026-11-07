package com.wanderstest3.pojarush

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

abstract class KeepAliveService : Service() {
    protected abstract val notificationId: Int
    protected abstract val title: Int
    protected abstract val body: Int

    private var wakeLock: PowerManager.WakeLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            manager.createNotificationChannel(
                NotificationChannel(CHANNEL, getString(R.string.keep_alive_channel), NotificationManager.IMPORTANCE_LOW).apply {
                    setShowBadge(false)
                }
            )
        }
        val open = PendingIntent.getActivity(
            this,
            notificationId,
            Intent.makeMainActivity(android.content.ComponentName(this, MainActivity::class.java)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val notification: Notification = NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(getString(title))
            .setContentText(getString(body))
            .setOngoing(true)
            .setSilent(true)
            .setContentIntent(open)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .build()
        val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE else 0
        runCatching { ServiceCompat.startForeground(this, notificationId, notification, type) }
            .onFailure {
                stopSelf()
                return START_NOT_STICKY
            }
        if (wakeLock == null) {
            wakeLock = (getSystemService(Context.POWER_SERVICE) as PowerManager)
                .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "pojarush:${javaClass.simpleName}")
                .apply {
                    setReferenceCounted(false)
                    acquire(WAKE_LIMIT_MS)
                }
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        runCatching { wakeLock?.release() }
        wakeLock = null
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "pojarush_keep_alive"
        private const val WAKE_LIMIT_MS = 6 * 60 * 60 * 1000L

        fun start(context: Context, service: Class<out KeepAliveService>) {
            runCatching {
                val intent = Intent(context, service)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) context.startForegroundService(intent) else context.startService(intent)
            }
        }

        fun stop(context: Context, service: Class<out KeepAliveService>) {
            runCatching { context.stopService(Intent(context, service)) }
        }
    }
}

class GameKeepAliveService : KeepAliveService() {
    override val notificationId = 3001
    override val title = R.string.keep_alive_game_title
    override val body = R.string.keep_alive_game_body
}

class PrepareKeepAliveService : KeepAliveService() {
    override val notificationId = 3002
    override val title = R.string.keep_alive_prepare_title
    override val body = R.string.keep_alive_prepare_body
}
