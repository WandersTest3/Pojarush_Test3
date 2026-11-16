package io.github.juanluci.starling;

import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;

public final class PojarushHooks {
    public static final String PLAY = "com.wanderstest3.pojarush.PLAY";
    public static final String AFTER = LauncherActivity.AFTER;

    private PojarushHooks() {}

    public static boolean nativeBuildReady(Context context) {
        return NativeMods.ready(context, com.wanderstest3.pojarush.BuildConfig.STARLING_VERSION);
    }

    public static String stamp(Context context) {
        return NativeMods.stamp(context);
    }

    public static Intent starlingGame(Context context) {
        return new Intent(context, GameActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
    }

    public static Intent restart(Context context, int pid) {
        return new Intent(context, LauncherActivity.class)
                .putExtra(LauncherActivity.AFTER, pid)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
    }

    public static Uri handedFile(Intent intent) {
        return LauncherActivity.handedFile(intent);
    }

    public static void receive(Activity activity, Uri file, Runnable done) {
        LauncherActivity.receive(activity, file, done);
    }
}
