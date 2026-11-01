package com.wanderstest3.pojarush

import android.annotation.SuppressLint
import android.app.Activity
import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import android.os.Bundle
import android.view.ViewGroup
import android.webkit.CookieManager
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.ProgressBar
import androidx.activity.ComponentActivity

class MicrosoftLoginActivity : ComponentActivity() {
    companion object {
        const val EXTRA_CODE = "code"
        const val EXTRA_ERROR = "error"
    }

    private var done = false

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val root = FrameLayout(this)
        val progress = ProgressBar(this).apply { isIndeterminate = true }
        val web = WebView(this)
        CookieManager.getInstance().removeAllCookies(null)
        web.settings.javaScriptEnabled = true
        web.settings.domStorageEnabled = true
        web.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean = handle(request.url)

            override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
                if (handle(Uri.parse(url))) view.stopLoading()
                progress.visibility = ProgressBar.VISIBLE
            }

            override fun onPageFinished(view: WebView, url: String) {
                progress.visibility = ProgressBar.GONE
            }
        }
        root.addView(web, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        root.addView(progress, FrameLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT, android.view.Gravity.CENTER))
        setContentView(root)
        web.loadUrl(MicrosoftAuth.authorizeUrl)
    }

    private fun handle(uri: Uri): Boolean {
        if (done || !uri.toString().startsWith(MicrosoftAuth.REDIRECT)) return false
        done = true
        val code = uri.getQueryParameter("code")
        val error = uri.getQueryParameter("error_description") ?: uri.getQueryParameter("error")
        setResult(
            if (code != null) Activity.RESULT_OK else Activity.RESULT_CANCELED,
            Intent().putExtra(EXTRA_CODE, code).putExtra(EXTRA_ERROR, error)
        )
        finish()
        return true
    }
}
