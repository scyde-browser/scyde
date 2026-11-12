package dev.scyde.browser

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import dev.scyde.browser.theme.ScydeMotion
import dev.scyde.browser.theme.ScydeSpace
import org.json.JSONArray
import org.json.JSONObject
import org.mozilla.geckoview.AllowOrDeny
import org.mozilla.geckoview.ContentBlocking
import org.mozilla.geckoview.GeckoResult
import org.mozilla.geckoview.GeckoRuntime
import org.mozilla.geckoview.GeckoRuntimeSettings
import org.mozilla.geckoview.GeckoSession
import org.mozilla.geckoview.GeckoSession.PromptDelegate
import org.mozilla.geckoview.GeckoSessionSettings
import org.mozilla.geckoview.WebExtension
import uniffi.scyde_core.OmniboxInput
import uniffi.scyde_core.PlacesStore
import uniffi.scyde_core.SettingsStore
import uniffi.scyde_core.VisitKind
import uniffi.scyde_core.bookmarksBarId
import uniffi.scyde_core.omniboxClassify
import uniffi.scyde_core.omniboxDeamp
import uniffi.scyde_core.omniboxDisplayUrl
import uniffi.scyde_core.omniboxSearchUrl
import uniffi.scyde_core.settingsSchema
import java.io.File

/** One tab: a GeckoSession and what the island shows for it. */
class Tab(val session: GeckoSession, val isPrivate: Boolean) {
    val id: Long = nextId++
    var url by mutableStateOf("")
    var title by mutableStateOf("")
    var progress by mutableIntStateOf(100)
    var canGoBack by mutableStateOf(false)
    var canGoForward by mutableStateOf(false)
    /** The page's theme-color as #RRGGBB (Aura tint, DESIGN.md §9). */
    var themeColor by mutableStateOf<String?>(null)
    /**
     * Full screen while reading (owner, 2026-11-12): scrolling down hides the island and the system
     * bars; scrolling up or a tap at the bottom edge brings them back.
     */
    var immersive by mutableStateOf(false)
    var bookmarked by mutableStateOf(false)
    var desktop by mutableStateOf(false)
    var typed = false
    private var lastY = 0

    val isWeb: Boolean get() = url.startsWith("https://") || url.startsWith("http://")
    val host: String? get() = if (isWeb) Uri.parse(url).host else null
    /** The registrable domain (eTLD+1) Shield switches are keyed by. */
    val site: String? get() = if (isWeb) omniboxDisplayUrl(url)?.domain else null

    fun scrolled(y: Int, threshold: Int, top: Int) {
        val delta = y - lastY
        when {
            y <= threshold / 2 -> immersive = false
            delta > threshold && y > top -> immersive = true
            delta < -threshold -> immersive = false
            else -> return // small moves keep the reference point, so slow scrolls still count
        }
        lastY = y
    }

    companion object {
        private var nextId = 1L
    }
}

/**
 * The browser (PLAN §9.2): one GeckoRuntime, tabs as GeckoSessions, the built-in Shield extension
 * (webRequestBlocking + WASM engine, ADR-0004) which also hosts the internal pages, and ScydeCore
 * (omnibox, history, settings) via UniFFI.
 */
class Browser(private val context: Context) {
    val tabs = mutableStateListOf<Tab>()
    var activeId by mutableStateOf<Long?>(null)
    var shieldReady by mutableStateOf(false)
    var blocked by mutableIntStateOf(0)
    var popupsBlocked by mutableIntStateOf(0)
    var appearance by mutableStateOf<Map<String, String>>(emptyMap())
    var filtersVersion by mutableStateOf("")
    /** Settings, Library and Shield open over the tabs in their own session, closed with × or back. */
    var sheetPage by mutableStateOf<String?>(null)
    var sheetSession by mutableStateOf<GeckoSession?>(null)
    /** Sites (eTLD+1) with Shield turned off (`shield.disabled_sites`). */
    var disabledSites by mutableStateOf<Set<String>>(emptySet())
    /** Requests blocked per page host since that host's last page load (from the extension). */
    val hostCounts = mutableStateMapOf<String, Int>()
    /** A short message that rises out of the island (DESIGN.md §8 Toast). */
    var toast by mutableStateOf<String?>(null)
    /** Peek: a long-pressed link shown in a preview session. */
    var peekUrl by mutableStateOf<String?>(null)
    var peekSession by mutableStateOf<GeckoSession?>(null)
    /** Find in page: "current of total", null while nothing was searched. */
    var findResult by mutableStateOf<Pair<Int, Int>?>(null)

    val active: Tab? get() = tabs.firstOrNull { it.id == activeId }

    private val main = Handler(Looper.getMainLooper())
    private val settingsFile = File(context.filesDir, "settings.json")
    val places: PlacesStore? = runCatching { PlacesStore.open(File(context.filesDir, "places.sqlite").path) }
        .onFailure { Log.e(TAG, "places", it) }.getOrNull()
    val settings: SettingsStore = SettingsStore.load(settingsFile.takeIf { it.exists() }?.readText())
    private var extension: WebExtension? = null
    private var port: WebExtension.Port? = null
    private val pending = mutableListOf<() -> Unit>()
    private val density = context.resources.displayMetrics.density

    val runtime: GeckoRuntime = shared ?: GeckoRuntime.create(
        context.applicationContext,
        GeckoRuntimeSettings.Builder()
            .contentBlocking(
                ContentBlocking.Settings.Builder()
                    .antiTracking(ContentBlocking.AntiTracking.STRICT)
                    .enhancedTrackingProtectionLevel(ContentBlocking.EtpLevel.STRICT)
                    .cookieBehavior(ContentBlocking.CookieBehavior.ACCEPT_FIRST_PARTY_AND_ISOLATE_OTHERS)
                    .build(),
            )
            .build(),
    ).also { shared = it }

    init {
        refreshAppearance()
        // Requests pass unfiltered until the engine is ready: the first loads wait for it, at most 5 s.
        main.postDelayed({ flushPending() }, 5000)
        runtime.webExtensionController.ensureBuiltIn(EXT_URI, EXT_ID).accept(
            { ext ->
                if (ext == null) return@accept
                extension = ext
                filtersVersion = ext.metaData.description?.substringAfter("filters ", "")?.removeSuffix(")") ?: ""
                ext.setMessageDelegate(messages, NATIVE_APP)
                for (t in tabs) t.session.webExtensionController.setMessageDelegate(ext, messages, NATIVE_APP)
                // Shield must also run in private tabs.
                runtime.webExtensionController.setAllowedInPrivateBrowsing(ext, true)
            },
            { e -> Log.e(TAG, "Shield extension failed", e); flushPending() },
        )
    }

    private var gateOpen = false

    private fun flushPending() {
        gateOpen = true
        val run = pending.toList()
        pending.clear()
        run.forEach { it() }
    }

    private fun whenReady(block: () -> Unit) {
        if (gateOpen) block() else pending += block
    }

    private fun str(id: Int, vararg args: Any): String = context.getString(id, *args)

    // ------------------------------------------------------------------ tabs

    fun internalUrl(page: String): String? = extension?.metaData?.baseUrl?.let { "${it}pages/$page/index.html" }

    /** `moz-extension://…/pages/settings/index.html` → "settings". */
    fun internalPage(url: String): String? {
        val base = extension?.metaData?.baseUrl ?: return null
        if (!url.startsWith(base + "pages/")) return null
        return url.removePrefix(base + "pages/").substringBefore('/')
    }

    private fun newSession(isPrivate: Boolean): GeckoSession =
        GeckoSession(GeckoSessionSettings.Builder().usePrivateMode(isPrivate).build())

    fun newTab(url: String? = null, session: GeckoSession? = null, activate: Boolean = true, isPrivate: Boolean = false): Tab {
        val s = session ?: newSession(isPrivate)
        val tab = Tab(s, isPrivate)
        attach(tab)
        // A session from onNewSession must stay closed: GeckoView opens it itself.
        if (session == null) s.open(runtime)
        extension?.let { s.webExtensionController.setMessageDelegate(it, messages, NATIVE_APP) }
        tabs += tab
        if (activate) activeId = tab.id
        if (session == null) {
            whenReady { s.loadUri(url ?: internalUrl("newtab") ?: "about:blank") }
        }
        return tab
    }

    fun close(tab: Tab) {
        val i = tabs.indexOfFirst { it.id == tab.id }
        if (i < 0) return
        tab.session.close()
        tabs.removeAt(i)
        if (activeId == tab.id) {
            // Stay in the same mode (private or not) when a tab is left in it.
            activeId = (tabs.lastOrNull { it.isPrivate == tab.isPrivate } ?: tabs.lastOrNull())?.id
        }
        if (tabs.isEmpty()) newTab()
    }

    fun closeAll(isPrivate: Boolean) {
        tabs.filter { it.isPrivate == isPrivate }.forEach { it.session.close() }
        tabs.removeAll { it.isPrivate == isPrivate }
        if (tabs.isEmpty()) newTab() else if (active == null) activeId = tabs.last().id
    }

    /** The neighbouring tab of the same mode; past the last one a new tab opens (like Safari). */
    fun switchTab(by: Int) {
        val current = active ?: return
        val list = tabs.filter { it.isPrivate == current.isPrivate }
        val i = list.indexOfFirst { it.id == current.id }
        val j = i + by
        when {
            i < 0 -> Unit
            j >= list.size -> newTab(isPrivate = current.isPrivate)
            j >= 0 -> activeId = list[j].id
        }
    }

    fun go(input: String, inNewTab: Boolean = false) {
        val target = when (val r = omniboxClassify(input)) {
            is OmniboxInput.Url -> r.url.let { u ->
                if (u.startsWith("scyde://")) internalUrl(u.removePrefix("scyde://").trim('/').substringBefore('/').ifEmpty { "newtab" }) ?: u else u
            }
            is OmniboxInput.Search -> omniboxSearchUrl(settings.searchTemplate(), r.query)
            OmniboxInput.Empty -> return
        }
        val tab = active
        if (inNewTab || tab == null) {
            newTab(target, isPrivate = tab?.isPrivate ?: false)
        } else {
            tab.typed = true
            tab.session.loadUri(target)
        }
    }

    fun openPage(page: String) {
        // Before the extension is loaded there is no page URL yet: try again once it is.
        val name = page.substringBefore('#')
        val fragment = page.substringAfter('#', "").let { if (it.isEmpty()) "" else "#$it" }
        val url = internalUrl(name)?.plus(fragment) ?: return if (!gateOpen) whenReady { openPage(page) } else Unit
        closeSheet()
        val s = GeckoSession()
        s.navigationDelegate = object : GeckoSession.NavigationDelegate {
            /** A link to a website opens in the current tab, not inside the sheet. */
            override fun onLoadRequest(session: GeckoSession, request: GeckoSession.NavigationDelegate.LoadRequest): GeckoResult<AllowOrDeny> {
                val u = request.uri
                if (u.startsWith("http://") || u.startsWith("https://")) {
                    main.post { closeSheet(); go(u) }
                    return GeckoResult.fromValue(AllowOrDeny.DENY)
                }
                return GeckoResult.fromValue(AllowOrDeny.ALLOW)
            }
        }
        s.open(runtime)
        extension?.let { s.webExtensionController.setMessageDelegate(it, messages, NATIVE_APP) }
        s.loadUri(url)
        sheetSession = s
        sheetPage = name
    }

    /** First run: the welcome sheet (theme, search engine, Shield) until it is finished or skipped. */
    val needsOnboarding: Boolean get() = settings.get("onboarding.completed") != "true"

    fun closeSheet() {
        sheetSession?.close()
        sheetSession = null
        sheetPage = null
    }

    /** Inline completion: the rest of the best host-prefix match from history. */
    fun suggestion(text: String): String? {
        if (text.isBlank()) return null
        val s = runCatching { places?.suggest(text, 1u)?.firstOrNull() }.getOrNull() ?: return null
        if (!s.hostPrefix) return null
        val host = s.url.substringAfter("://").removePrefix("www.").substringBefore('/').substringBefore('?')
        val typed = text.lowercase()
        return if (host.startsWith(typed) && host.length > typed.length) host.substring(typed.length) else null
    }

    // ------------------------------------------------------------- page tools

    fun reloadOrStop() {
        val tab = active ?: return
        if (tab.progress in 1..99) tab.session.stop() else tab.session.reload()
    }

    fun toggleBookmark() {
        val tab = active ?: return
        val p = places ?: return
        if (!tab.isWeb) return
        runCatching {
            if (tab.bookmarked) {
                p.unbookmarkUrl(tab.url)
                show(str(R.string.toast_unbookmarked))
            } else {
                p.addBookmark(bookmarksBarId(), tab.url, tab.title, System.currentTimeMillis() / 1000)
                show(str(R.string.toast_bookmarked))
            }
            tab.bookmarked = p.isBookmarked(tab.url)
        }
    }

    fun toggleDesktop() {
        val tab = active ?: return
        tab.desktop = !tab.desktop
        tab.session.settings.userAgentMode =
            if (tab.desktop) GeckoSessionSettings.USER_AGENT_MODE_DESKTOP else GeckoSessionSettings.USER_AGENT_MODE_MOBILE
        tab.session.settings.viewportMode =
            if (tab.desktop) GeckoSessionSettings.VIEWPORT_MODE_DESKTOP else GeckoSessionSettings.VIEWPORT_MODE_MOBILE
        tab.session.reload()
    }

    fun copyLink(url: String? = active?.takeIf { it.isWeb }?.url) {
        if (url == null) return
        context.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("url", url))
        show(str(R.string.toast_link_copied))
    }

    fun share(url: String? = active?.takeIf { it.isWeb }?.url) {
        if (url == null) return
        val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, url)
        context.startActivity(Intent.createChooser(send, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    fun find(text: String?, backwards: Boolean = false) {
        val finder = active?.session?.finder ?: return
        finder.displayFlags = GeckoSession.FINDER_DISPLAY_HIGHLIGHT_ALL or GeckoSession.FINDER_DISPLAY_DIM_PAGE or
            GeckoSession.FINDER_DISPLAY_DRAW_LINK_OUTLINE
        if (text != null && text.isEmpty()) {
            finder.clear()
            findResult = null
            return
        }
        finder.find(text, if (backwards) GeckoSession.FINDER_FIND_BACKWARDS else 0).accept { r ->
            if (r != null) findResult = r.current to r.total
        }
    }

    fun endFind() {
        active?.session?.finder?.clear()
        findResult = null
    }

    fun show(message: String) {
        toast = message
        main.removeCallbacks(hideToast)
        main.postDelayed(hideToast, ScydeMotion.toastDismiss.toLong())
    }

    private val hideToast = Runnable { toast = null }

    fun openPeek(url: String, isPrivate: Boolean) {
        closePeek()
        val s = newSession(isPrivate)
        s.open(runtime)
        s.loadUri(url)
        peekSession = s
        peekUrl = url
    }

    fun closePeek() {
        peekSession?.close()
        peekSession = null
        peekUrl = null
    }

    // ------------------------------------------------------------- Shield

    fun shieldOn(tab: Tab?): Boolean = tab?.site?.let { it !in disabledSites } ?: true

    /** Turns Shield off or on for a site, tells the extension, saves it and reloads that site's tabs. */
    fun setShield(site: String, on: Boolean) {
        if (site.isEmpty()) return
        disabledSites = if (on) disabledSites - site else disabledSites + site
        runCatching { settings.set("shield.disabled_sites", JSONArray(disabledSites.sorted()).toString()) }
        runCatching { settingsFile.writeText(settings.toJson()) }
        sendAllowlist()
        tabs.filter { it.site == site }.forEach { it.session.reload() }
        show(str(if (on) R.string.toast_shield_on else R.string.toast_shield_off, site))
    }

    private fun sendAllowlist() {
        port?.postMessage(JSONObject().put("type", "allowlist").put("sites", JSONArray(disabledSites.sorted())))
    }

    /** Blocked requests on the active tab's page (null when the page is not a website). */
    fun blockedOn(tab: Tab?): Int? = tab?.host?.let { hostCounts[it] ?: 0 }

    private fun attach(tab: Tab) {
        val s = tab.session
        s.navigationDelegate = object : GeckoSession.NavigationDelegate {
            override fun onLocationChange(
                session: GeckoSession,
                url: String?,
                perms: MutableList<GeckoSession.PermissionDelegate.ContentPermission>,
                hasUserGesture: Boolean,
            ) {
                tab.url = url ?: ""
                tab.themeColor = null
                tab.bookmarked = tab.isWeb && runCatching { places?.isBookmarked(tab.url) }.getOrNull() == true
            }

            /** De-AMP: a Google AMP viewer URL loads the publisher's page, so the island shows the real domain. */
            override fun onLoadRequest(session: GeckoSession, request: GeckoSession.NavigationDelegate.LoadRequest): GeckoResult<AllowOrDeny>? {
                val target = omniboxDeamp(request.uri) ?: return null
                main.post { session.loadUri(target) }
                return GeckoResult.fromValue(AllowOrDeny.DENY)
            }

            override fun onCanGoBack(session: GeckoSession, canGoBack: Boolean) {
                tab.canGoBack = canGoBack
            }

            override fun onCanGoForward(session: GeckoSession, canGoForward: Boolean) {
                tab.canGoForward = canGoForward
            }

            /** A popup that passed Gecko's gesture check opens as a tab and keeps its opener. */
            override fun onNewSession(session: GeckoSession, uri: String): GeckoResult<GeckoSession> {
                // Called on the UI thread: the tab and its delegates exist before Gecko opens the session.
                val child = newSession(tab.isPrivate)
                newTab(session = child, isPrivate = tab.isPrivate)
                return GeckoResult.fromValue(child)
            }
        }
        s.progressDelegate = object : GeckoSession.ProgressDelegate {
            override fun onProgressChange(session: GeckoSession, progress: Int) {
                tab.progress = progress
            }

            override fun onPageStop(session: GeckoSession, success: Boolean) {
                tab.progress = 100
                val url = tab.url
                // Private tabs leave no history.
                if (success && !tab.isPrivate && (url.startsWith("https://") || url.startsWith("http://"))) {
                    val kind = if (tab.typed) VisitKind.TYPED else VisitKind.LINK
                    tab.typed = false
                    runCatching { places?.recordVisit(url, tab.title.ifEmpty { null }, kind, System.currentTimeMillis() / 1000) }
                }
            }
        }
        s.contentDelegate = object : GeckoSession.ContentDelegate {
            override fun onTitleChange(session: GeckoSession, title: String?) {
                tab.title = title ?: ""
                val url = tab.url
                if (title != null && !tab.isPrivate && url.startsWith("http")) runCatching { places?.setTitle(url, title) }
            }

            override fun onCloseRequest(session: GeckoSession) {
                close(tab)
            }

            /** Long-press on a link: Peek (preview and actions). */
            override fun onContextMenu(session: GeckoSession, screenX: Int, screenY: Int, element: GeckoSession.ContentDelegate.ContextElement) {
                val link = element.linkUri ?: return
                if (link.startsWith("http://") || link.startsWith("https://")) openPeek(link, tab.isPrivate)
            }
        }
        s.scrollDelegate = object : GeckoSession.ScrollDelegate {
            override fun onScrollChanged(session: GeckoSession, scrollX: Int, scrollY: Int) {
                tab.scrolled(scrollY, (ScydeSpace.s12.value * density).toInt(), (ScydeSpace.s64.value * density).toInt())
            }
        }
        // Gecko blocks window.open without a gesture and asks here: always deny and count (PLAN §5.5).
        s.promptDelegate = object : PromptDelegate {
            override fun onPopupPrompt(session: GeckoSession, prompt: PromptDelegate.PopupPrompt): GeckoResult<PromptDelegate.PromptResponse> {
                popupsBlocked++
                return GeckoResult.fromValue(prompt.confirm(AllowOrDeny.DENY))
            }
        }
    }

    // -------------------------------------------------------------- settings

    private fun refreshAppearance() {
        appearance = listOf("theme", "accent", "density", "reduce_motion").associateWith { key ->
            settings.get("appearance.$key")?.trim('"') ?: ""
        }
        disabledSites = runCatching {
            val a = JSONArray(settings.get("shield.disabled_sites") ?: "[]")
            (0 until a.length()).map { a.getString(it) }.toSet()
        }.getOrDefault(emptySet())
    }

    private fun saveSettings() {
        runCatching { settingsFile.writeText(settings.toJson()) }
        val before = disabledSites
        refreshAppearance()
        if (before != disabledSites) sendAllowlist()
    }

    // ------------------------------------------------------------- messaging

    /** Background script (ready, counters, allowlist port), content scripts and internal pages. */
    private val messages = object : WebExtension.MessageDelegate {
        override fun onConnect(port: WebExtension.Port) {
            this@Browser.port = port
            port.setDelegate(object : WebExtension.PortDelegate {
                override fun onPortMessage(message: Any, port: WebExtension.Port) {}

                override fun onDisconnect(port: WebExtension.Port) {
                    if (this@Browser.port === port) this@Browser.port = null
                }
            })
            main.post { sendAllowlist() }
        }

        override fun onMessage(nativeApp: String, message: Any, sender: WebExtension.MessageSender): GeckoResult<Any>? {
            val json = message as? JSONObject ?: return null
            when (json.optString("type")) {
                "ready" -> {
                    main.post { shieldReady = true; flushPending() }
                    return GeckoResult.fromValue(JSONObject().put("ok", true))
                }
                "shield.count" -> {
                    val hosts = json.optJSONObject("hosts")
                    main.post {
                        blocked = json.optInt("blocked")
                        hosts?.keys()?.forEach { h -> hostCounts[h] = hosts.optInt(h) }
                    }
                    return null
                }
                "theme" -> {
                    val color = json.optString("color").takeIf { Regex("^#[0-9A-Fa-f]{6}$").matches(it) }
                    val session = sender.session
                    main.post { tabs.firstOrNull { it.session === session }?.themeColor = color }
                    return null
                }
                "error" -> {
                    Log.e(TAG, "Shield: ${json.optString("error")}")
                    main.post { flushPending() }
                    return null
                }
            }
            val method = json.optString("method")
            val base = extension?.metaData?.baseUrl
            val trusted = base != null && sender.url.startsWith(base + "pages/") &&
                sender.environmentType == WebExtension.MessageSender.ENV_TYPE_EXTENSION
            val reply = JSONObject().put("id", json.opt("id"))
            if (!trusted) return GeckoResult.fromValue(reply.put("ok", false).put("error", "untrusted origin"))
            if (method !in METHODS) return GeckoResult.fromValue(reply.put("ok", false).put("error", "method not allowed"))
            val result = GeckoResult<Any>()
            main.post {
                try {
                    result.complete(reply.put("ok", true).put("result", handle(method, json.optJSONObject("params") ?: JSONObject())))
                } catch (e: Exception) {
                    result.complete(reply.put("ok", false).put("error", e.message ?: "error"))
                }
            }
            return result
        }
    }

    private fun visits(list: List<uniffi.scyde_core.HistoryEntry>) = JSONArray().apply {
        list.forEach { put(JSONObject().put("url", it.url).put("title", it.title).put("lastVisit", it.lastVisit).put("visitCount", it.visitCount.toLong())) }
    }

    /** Same contract as web/packages/bridge/src/api.ts. */
    private fun handle(method: String, p: JSONObject): Any = when (method) {
        "newtab.state" -> JSONObject()
            .put("shield", JSONObject().put("blocked", blocked + popupsBlocked).put("bytesSaved", 0).put("msSaved", 0))
            .put("recent", visits(places?.history(null, 6u, 0u) ?: emptyList()))
            .put("topSites", JSONArray().apply { places?.topSites(8u)?.forEach { put(JSONObject().put("url", it.url).put("title", it.title).put("pinned", it.pinned)) } })
            .put("storiesEnabled", settings.get("newtab.top_stories") == "true")
        "navigate" -> {
            closeSheet()
            go(p.optString("input"), p.optBoolean("newTab"))
            JSONObject().put("ok", true)
        }
        "settings.schema" -> JSONObject().put("schema", JSONObject(settingsSchema()))
        "settings.get" -> JSONObject().put("settings", JSONObject(settings.toJson()))
            .put("issues", JSONArray().apply { settings.loadIssues().forEach { put(JSONObject().put("path", it.path).put("message", it.message)) } })
        "settings.set" -> {
            val v = p.opt("value")
            val text = if (v is String) JSONObject.quote(v) else v.toString()
            try {
                settings.set(p.optString("path"), text)
                saveSettings()
                // The welcome page is finished: close its sheet.
                if (p.optString("path") == "onboarding.completed" && v == true) closeSheet()
                JSONObject().put("ok", true)
            } catch (e: Exception) {
                JSONObject().put("ok", false).put("error", e.message)
            }
        }
        "settings.reset" -> {
            settings.reset(p.optString("path"))
            saveSettings()
            JSONObject().put("ok", true)
        }
        "history.query" -> JSONObject().put(
            "entries",
            visits(places?.history(p.optString("query").ifEmpty { null }, p.optInt("limit", 50).toUInt(), p.optInt("offset", 0).toUInt()) ?: emptyList()),
        )
        "history.delete" -> { places?.deleteUrl(p.optString("url")); JSONObject().put("ok", true) }
        "history.clear" -> { places?.clearHistory(); JSONObject().put("ok", true) }
        "bookmarks.children" -> JSONObject().put(
            "items",
            JSONArray().apply {
                places?.children(p.optLong("parent", bookmarksBarId()))?.forEach {
                    put(JSONObject().put("id", it.id).put("parent", it.parent).put("isFolder", it.isFolder).put("url", it.url ?: JSONObject.NULL).put("title", it.title))
                }
            },
        )
        "bookmarks.remove" -> { places?.removeBookmark(p.optLong("id")); JSONObject().put("ok", true) }
        "shield.summary" -> {
            val today = JSONObject().put("blocked", blocked + popupsBlocked).put("bytesSaved", 0).put("msSaved", 0)
            val sites = JSONArray().apply {
                disabledSites.sorted().forEach { put(JSONObject().put("host", it).put("enabled", false).put("blocked", 0)) }
            }
            JSONObject().put("today", today).put("total", today).put("filtersVersion", filtersVersion).put("sites", sites)
        }
        "shield.setSite" -> {
            setShield(p.optString("host"), p.optBoolean("enabled", true))
            JSONObject().put("ok", true)
        }
        else -> JSONObject().put("ok", false).put("error", "method not allowed")
    }

    companion object {
        private const val TAG = "SCYDE"
        const val EXT_ID = "shield@scyde.dev"
        const val EXT_URI = "resource://android/assets/extension/"
        const val NATIVE_APP = "scyde"
        val METHODS = setOf(
            "newtab.state", "navigate", "settings.schema", "settings.get", "settings.set", "settings.reset",
            "history.query", "history.delete", "history.clear", "bookmarks.children", "bookmarks.remove",
            "shield.summary", "shield.setSite",
        )

        @Volatile private var shared: GeckoRuntime? = null
    }
}
