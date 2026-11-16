package com.wanderstest3.pojarush.starling;

import android.app.Activity;
import android.net.Uri;
import android.os.Bundle;
import android.widget.Toast;
import com.wanderstest3.pojarush.R;
import io.github.juanluci.starling.PojarushHooks;

public final class ImportActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Uri file = PojarushHooks.handedFile(getIntent());
        if (file == null) {
            finish();
            return;
        }
        Toast.makeText(this, R.string.starling_import_started, Toast.LENGTH_SHORT).show();
        PojarushHooks.receive(this, file, new Runnable() {
            @Override
            public void run() {
                Toast.makeText(ImportActivity.this, R.string.starling_import_done, Toast.LENGTH_LONG).show();
                finish();
            }
        });
    }
}
