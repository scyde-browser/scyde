package dev.scyde.spike.gecko

import android.app.Activity
import android.os.Bundle
import android.util.Log
import org.json.JSONObject
import org.mozilla.geckoview.BuildConfig as GeckoBuildConfig
import org.mozilla.geckoview.ContentBlocking
import org.mozilla.geckoview.GeckoResult
import org.mozilla.geckoview.GeckoRuntime
import org.mozilla.geckoview.GeckoRuntimeSettings
import org.mozilla.geckoview.GeckoSession
import org.mozilla.geckoview.GeckoView
import org.mozilla.geckoview.WebExtension

/**
 * Spike S3: GeckoView + built-in Shield extension (MV2 webRequestBlocking + WASM engine) + native messaging.
 * Intent extras: "url" (page to open first), "internal" (true = open the extension's internal page afterwards).
 */
class MainActivity : Activity() {
    private lateinit var session: GeckoSession

    companion object {
        @Volatile var runtime: GeckoRuntime? = null
        const val EXT_ID = "shield@spike.scyde.dev"
        const val NATIVE_APP = "scyde"
    }

    private val delegate = object : WebExtension.MessageDelegate {
        override fun onMessage(nativeApp: String, message: Any, sender: WebExtension.MessageSender): GeckoResult<Any>? {
            val json = message as? JSONObject ?: return null
            val type = json.optString("type")
            ResultBus.put(type, json)
            return GeckoResult.fromValue(JSONObject().put("ok", true).put("from", "android").put("echo", type))
        }
    }

    private var canGoBack = false

    @Deprecated("Spike: simple back handling")
    override fun onBackPressed() {
        if (canGoBack) session.goBack() else super.onBackPressed()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val view = GeckoView(this)
        setContentView(view)

        val rt = runtime ?: GeckoRuntime.create(
            applicationContext,
            GeckoRuntimeSettings.Builder()
                .contentBlocking(
                    ContentBlocking.Settings.Builder()
                        .antiTracking(ContentBlocking.AntiTracking.STRICT)
                        .enhancedTrackingProtectionLevel(ContentBlocking.EtpLevel.STRICT)
                        .cookieBehavior(ContentBlocking.CookieBehavior.ACCEPT_FIRST_PARTY_AND_ISOLATE_OTHERS)
                        .build(),
                )
                .build(),
        ).also { runtime = it }

        ResultBus.put("gecko", JSONObject().put("version", GeckoBuildConfig.MOZ_APP_VERSION))

        session = GeckoSession()
        session.navigationDelegate = object : GeckoSession.NavigationDelegate {
            override fun onCanGoBack(session: GeckoSession, value: Boolean) { canGoBack = value }
        }
        session.open(rt)
        view.setSession(session)

        // Launcher start (no extras) = manual S1 test: open the internal page with sign-in links.
        val url: String? = intent.getStringExtra("url")
        val openInternal = intent.getBooleanExtra("internal", url == null)

        rt.webExtensionController
            .ensureBuiltIn("resource://android/assets/extension/", EXT_ID)
            .accept(
                { ext ->
                    if (ext == null) return@accept
                    ext.setMessageDelegate(delegate, NATIVE_APP)
                    session.webExtensionController.setMessageDelegate(ext, delegate, NATIVE_APP)
                    ResultBus.put("extension", JSONObject().put("id", ext.id).put("baseUrl", ext.metaData.baseUrl))
                    Thread {
                        // Wait until the WASM engine is built, otherwise the first page would load unfiltered.
                        ResultBus.await("ready", 60)
                        if (url != null) {
                            runOnUiThread { session.loadUri(url) }
                            if (openInternal) ResultBus.await("fixture", 90)
                        }
                        if (openInternal) runOnUiThread { session.loadUri(ext.metaData.baseUrl + "internal.html") }
                    }.start()
                },
                { e -> Log.e("SCYDE", "extension install failed", e); ResultBus.put("extension-error", JSONObject().put("error", e.toString())) },
            )
    }
}
