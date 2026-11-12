package dev.scyde.browser

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.ActivityNotFoundException
import android.content.ContentValues
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.util.Base64
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.toArgb
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
import org.mozilla.geckoview.WebRequestError
import org.mozilla.geckoview.WebResponse
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

/** A Space: a named group of tabs with its own Aura colour (PLAN Phase 6 #8, ADR-0013). */
data class Space(val id: String, val name: String, val color: String) {
    companion object {
        val COLORS = listOf("ember", "sky", "mint", "lavender", "periwinkle", "onyx")
    }
}

/** One tab: a GeckoSession and what the island shows for it. */
class Tab(firstSession: GeckoSession, val isPrivate: Boolean, firstSpace: String) {
    val id: Long = nextId++
    /** Replaced by a new, closed session while the tab sleeps (that frees its memory). */
    var session by mutableStateOf(firstSession)
    var spaceId by mutableStateOf(firstSpace)
    var pinned by mutableStateOf(false)
    /** Asleep: no page in memory; it comes back (with its history) when the tab is opened. */
    var sleeping by mutableStateOf(false)
    var lastActive = System.currentTimeMillis()
    /** The latest session state (history), restored when a sleeping tab wakes. */
    var savedState: GeckoSession.SessionState? = null
    /** What a restored tab loads when it wakes (it has no saved state). */
    var pendingUrl: String? = null
    /** The next finished load is a reload or a wake, not a visit (history counts stay honest). */
    var skipVisit = false
    /** The page as it last looked, for the tab grid. */
    var thumbnail by mutableStateOf<androidx.compose.ui.graphics.ImageBitmap?>(null)
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
    /** A video (or any element) the page put in full screen: island and system bars go away. */
    var fullscreen by mutableStateOf(false)
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
class Browser(appContext: Context) {
    private val context: Context = appContext.applicationContext
    val tabs = mutableStateListOf<Tab>()
    private var activeState by mutableStateOf<Long?>(null)
    var activeId: Long?
        get() = activeState
        set(v) {
            // Stamp the tab being left too, or a long-read tab would fall asleep the moment it is left.
            active?.lastActive = System.currentTimeMillis()
            if (v != activeState) snapshotActive()
            activeState = v
            val tab = active ?: return
            tab.lastActive = System.currentTimeMillis()
            wake(tab)
            if (!tab.isPrivate) spaceId = tab.spaceId
            saveSession()
        }
    /** Spaces (tab groups with their own colour); private tabs belong to none. */
    val spaces = mutableStateListOf(Space("personal", context.getString(R.string.space_default), "ember"))
    var spaceId by mutableStateOf("personal")
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
    /** What the page asks right now (alert, confirm, select, sign-in, permission…), shown by PromptHost. */
    var prompt: PagePrompt? by mutableStateOf(null)
        set(v) {
            // A new question while one is open: the open one is answered as cancelled.
            val old = field
            field = v
            if (old != null && old !== v) old.cancel()
        }
    /** Set by the Activity: the system file picker (mime types, multiple) → chosen files. */
    var pickFiles: ((Array<String>, Boolean, (List<Uri>) -> Unit) -> Unit)? = null
    /** Set by the Activity: Android runtime permissions → granted. */
    var requestAndroidPermissions: ((Array<String>, (Boolean) -> Unit) -> Unit)? = null
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
    private val afterExtension = mutableListOf<() -> Unit>()

    val runtime: GeckoRuntime = shared ?: GeckoRuntime.create(
        context.applicationContext,
        GeckoRuntimeSettings.Builder()
            // Page console in logcat only when asked for: `adb shell setprop log.tag.SCYDE DEBUG` (CI).
            .consoleOutput(Log.isLoggable(TAG, Log.DEBUG))
            .contentBlocking(
                ContentBlocking.Settings.Builder()
                    .antiTracking(ContentBlocking.AntiTracking.STRICT)
                    .enhancedTrackingProtectionLevel(ContentBlocking.EtpLevel.STRICT)
                    .cookieBehavior(ContentBlocking.CookieBehavior.ACCEPT_FIRST_PARTY_AND_ISOLATE_OTHERS)
                    .build(),
            )
            .build(),
    ).also { shared = it }

    private val sessionFile = File(context.filesDir, "session.json")
    private var restoring = false

    init {
        refreshAppearance()
        main.postDelayed(object : Runnable {
            override fun run() {
                sleepIdleTabs()
                main.postDelayed(this, 60_000)
            }
        }, 60_000)
        // Requests pass unfiltered until the engine is ready: the first loads wait for it, at most 5 s.
        main.postDelayed({ flushPending() }, 5000)
        runtime.webExtensionController.ensureBuiltIn(EXT_URI, EXT_ID).accept(
            { ext ->
                if (ext == null) return@accept
                extension = ext
                filtersVersion = ext.metaData.description?.substringAfter("filters ", "")?.removeSuffix(")") ?: ""
                ext.setMessageDelegate(messages, NATIVE_APP)
                for (t in tabs) t.session.webExtensionController.setMessageDelegate(ext, messages, NATIVE_APP)
                afterExtension.toList().also { afterExtension.clear() }.forEach { it() }
                // Shield must also run in private tabs.
                // Only when needed: the call reloads the extension (and its engine).
                if (!ext.metaData.allowedInPrivateBrowsing) runtime.webExtensionController.setAllowedInPrivateBrowsing(ext, true)
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
    fun string(id: Int, vararg args: Any): String = context.getString(id, *args)
    val appContext: Context get() = context

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
        val tab = Tab(s, isPrivate, spaceId)
        attach(tab)
        // A session from onNewSession must stay closed: GeckoView opens it itself.
        if (session == null) s.open(runtime)
        extension?.let { s.webExtensionController.setMessageDelegate(it, messages, NATIVE_APP) }
        tabs += tab
        if (activate) activeId = tab.id else saveSession()
        if (session == null) {
            whenReady {
                // New Tab lives in the extension: if it is not installed yet (slow first start), wait
                // for it instead of showing about:blank.
                val target = url ?: internalUrl("newtab")
                if (target != null) s.loadUri(target) else afterExtension += { s.loadUri(internalUrl("newtab") ?: "about:blank") }
            }
        }
        return tab
    }

    fun close(tab: Tab) {
        val i = tabs.indexOfFirst { it.id == tab.id }
        if (i < 0) return
        tab.session.close()
        tabs.removeAt(i)
        if (activeState == tab.id) {
            // Stay in the same mode and Space when a tab is left in it.
            val same = visibleTabs(tab.isPrivate).ifEmpty { visibleTabs(false) }
            activeState = null
            same.maxByOrNull { it.lastActive }?.let { activeId = it.id }
        }
        if (active == null) newTab()
        saveSession()
    }

    /** Closes the private tabs, or the unpinned tabs of the current Space. */
    fun closeAll(isPrivate: Boolean) {
        val doomed = tabs.filter { if (isPrivate) it.isPrivate else !it.isPrivate && it.spaceId == spaceId && !it.pinned }
        doomed.forEach { it.session.close() }
        tabs.removeAll { it in doomed }
        if (active == null) {
            val left = visibleTabs(false)
            if (left.isNotEmpty()) activeId = left.first().id else newTab()
        }
        saveSession()
    }

    /** The GeckoView on screen (set by the UI), for page thumbnails. */
    var view: java.lang.ref.WeakReference<org.mozilla.geckoview.GeckoView>? = null

    /** Keeps a small picture of the active page for its card in the tab grid. */
    fun snapshotActive() {
        val tab = active ?: return
        val v = view?.get() ?: return
        if (sheetSession != null || tab.sleeping || v.session !== tab.session || v.width == 0) return
        runCatching {
            v.capturePixels().accept({ bitmap ->
                if (bitmap != null) {
                    val w = bitmap.width / 4
                    val h = bitmap.height / 4
                    if (w > 0 && h > 0) tab.thumbnail = android.graphics.Bitmap.createScaledBitmap(bitmap, w, h, true).asImageBitmap()
                }
            }, { })
        }
    }

    /** What the island counts and swipes through: the active mode and, for normal tabs, the Space. */
    fun visibleTabs(isPrivate: Boolean): List<Tab> {
        val list = tabs.filter { it.isPrivate == isPrivate && (isPrivate || it.spaceId == spaceId) }
        return list.filter { it.pinned } + list.filter { !it.pinned }
    }

    fun togglePin(tab: Tab) {
        tab.pinned = !tab.pinned
        saveSession()
    }

    // ---------------------------------------------------------------- Spaces

    val currentSpace: Space get() = spaces.firstOrNull { it.id == spaceId } ?: spaces.first()

    fun switchSpace(id: String) {
        if (spaces.none { it.id == id }) return
        spaceId = id
        val last = tabs.filter { !it.isPrivate && it.spaceId == id }.maxByOrNull { it.lastActive }
        if (last != null) activeId = last.id else newTab()
    }

    fun addSpace(name: String) {
        val used = spaces.map { it.color }.toSet()
        val color = Space.COLORS.firstOrNull { it !in used } ?: Space.COLORS[spaces.size % Space.COLORS.size]
        val space = Space(java.util.UUID.randomUUID().toString(), name.trim().ifEmpty { str(R.string.space_new) }, color)
        spaces += space
        switchSpace(space.id)
    }

    fun renameSpace(id: String, name: String) {
        val i = spaces.indexOfFirst { it.id == id }
        if (i < 0 || name.isBlank()) return
        spaces[i] = spaces[i].copy(name = name.trim())
        saveSession()
    }

    fun recolorSpace(id: String, color: String) {
        val i = spaces.indexOfFirst { it.id == id }
        if (i < 0) return
        spaces[i] = spaces[i].copy(color = color)
        saveSession()
    }

    /** Deletes a Space; its tabs move to the first remaining Space. */
    fun deleteSpace(id: String) {
        if (spaces.size < 2) return
        spaces.removeAll { it.id == id }
        val target = spaces.first().id
        tabs.filter { it.spaceId == id }.forEach { it.spaceId = target }
        if (spaceId == id) switchSpace(target) else saveSession()
    }

    fun move(tab: Tab, space: String) {
        tab.spaceId = space
        if (tab.id == activeId) spaceId = space
        saveSession()
    }

    // ----------------------------------------------------- sleeping tabs

    private fun sleepIdleTabs() {
        val minutes = settings.get("tabs.sleep_after_minutes")?.toIntOrNull() ?: 15
        if (minutes <= 0) return
        val limit = System.currentTimeMillis() - minutes * 60_000L
        tabs.filter { it.id != activeId && !it.sleeping && it.lastActive < limit }.forEach { sleep(it) }
    }

    /** Frees a tab's page: keeps its history (SessionState) and swaps in a new, closed session. */
    private fun sleep(tab: Tab) {
        if (tab.sleeping) return
        tab.pendingUrl = tab.url
        tab.session.close()
        tab.session = newSession(tab.isPrivate)
        if (tab.desktop) {
            tab.session.settings.userAgentMode = GeckoSessionSettings.USER_AGENT_MODE_DESKTOP
            tab.session.settings.viewportMode = GeckoSessionSettings.VIEWPORT_MODE_DESKTOP
        }
        attach(tab)
        tab.sleeping = true
    }

    private fun wake(tab: Tab) {
        if (!tab.sleeping) return
        tab.sleeping = false
        tab.skipVisit = true
        val s = tab.session
        if (!s.isOpen) s.open(runtime)
        extension?.let { s.webExtensionController.setMessageDelegate(it, messages, NATIVE_APP) }
        val state = tab.savedState
        val url = tab.pendingUrl
        tab.pendingUrl = null
        whenReady {
            if (state != null) s.restoreState(state) else if (url != null) s.loadUri(url)
        }
    }

    // ------------------------------------------------------------ session

    /** Saves the open (non-private) tabs and Spaces; private tabs are never written anywhere. */
    fun saveSession() {
        if (restoring) return
        val json = JSONObject()
            .put("version", 1)
            .put("space", spaceId)
            .put("active", tabs.filter { !it.isPrivate }.indexOfFirst { it.id == activeId })
            .put("spaces", JSONArray().apply { spaces.forEach { put(JSONObject().put("id", it.id).put("name", it.name).put("color", it.color)) } })
            .put(
                "tabs",
                JSONArray().apply {
                    tabs.filter { !it.isPrivate }.forEach {
                        put(JSONObject().put("url", it.url.ifEmpty { it.pendingUrl ?: "" }).put("title", it.title).put("pinned", it.pinned).put("space", it.spaceId))
                    }
                },
            )
        runCatching { sessionFile.writeText(json.toString()) }
    }

    /**
     * Restores the last session as sleeping tabs (only the active one loads). False when there is
     * nothing to restore; `startup.open = newtab` keeps only pinned tabs.
     */
    fun restoreSession(): Boolean {
        val json = runCatching { JSONObject(sessionFile.readText()) }.getOrNull() ?: return false
        val mode = settings.get("startup.open")?.trim('"') ?: "restore"
        json.optJSONArray("spaces")?.let { a ->
            val list = (0 until a.length()).map { a.getJSONObject(it) }.map { Space(it.optString("id"), it.optString("name"), it.optString("color", "ember")) }
            if (list.isNotEmpty()) { spaces.clear(); spaces.addAll(list) }
        }
        spaceId = json.optString("space").takeIf { id -> spaces.any { it.id == id } } ?: spaces.first().id
        val saved = json.optJSONArray("tabs") ?: return false
        restoring = true
        val activeIndex = json.optInt("active", -1)
        var activeTab: Tab? = null
        for (i in 0 until saved.length()) {
            val o = saved.getJSONObject(i)
            val pinned = o.optBoolean("pinned")
            val url = o.optString("url")
            if (url.isEmpty() || (mode != "restore" && !pinned)) continue
            val space = o.optString("space").takeIf { id -> spaces.any { it.id == id } } ?: spaces.first().id
            val tab = Tab(newSession(false), false, space)
            attach(tab)
            tab.url = url
            tab.title = o.optString("title")
            tab.pinned = pinned
            tab.pendingUrl = url
            tab.sleeping = true
            tabs += tab
            if (i == activeIndex) activeTab = tab
        }
        restoring = false
        if (tabs.isEmpty()) return false
        if (mode != "restore") newTab() else activeId = (activeTab ?: tabs.firstOrNull { it.spaceId == spaceId } ?: tabs.first()).id
        return true
    }

    /** The neighbouring tab of the same mode; past the last one a new tab opens (like Safari). */
    fun switchTab(by: Int) {
        val current = active ?: return
        val list = visibleTabs(current.isPrivate)
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
        // `?embedded`: the sheet has its own title bar, the page hides its heading (welcome keeps its header).
        val query = if (name == "welcome") "" else "?embedded"
        val url = internalUrl(name)?.plus(query + fragment) ?: return if (!gateOpen) whenReady { openPage(page) } else Unit
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
        // Closing the welcome sheet any way (×, back, Skip) finishes onboarding.
        if (sheetPage == "welcome" && needsOnboarding) {
            runCatching { settings.set("onboarding.completed", "true") }
            runCatching { settingsFile.writeText(settings.toJson()) }
        }
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
        if (tab.progress in 1..99) tab.session.stop() else { tab.skipVisit = true; tab.session.reload() }
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

    // ------------------------------------------------- downloads, external apps, errors

    private fun isExternal(uri: String): Boolean {
        val scheme = uri.substringBefore(':', "").lowercase()
        return scheme.isNotEmpty() && scheme !in INTERNAL_SCHEMES
    }

    private fun openExternal(uri: String, session: GeckoSession) {
        val intent = runCatching {
            if (uri.startsWith("intent:", ignoreCase = true)) {
                // A page must not reach a private component: only what any browser link may open.
                Intent.parseUri(uri, Intent.URI_INTENT_SCHEME).apply {
                    addCategory(Intent.CATEGORY_BROWSABLE)
                    component = null
                    selector = null
                }
            } else {
                Intent(Intent.ACTION_VIEW, Uri.parse(uri)).addCategory(Intent.CATEGORY_BROWSABLE)
            }
        }.getOrNull() ?: return
        try {
            context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (_: ActivityNotFoundException) {
            val fallback = intent.getStringExtra("browser_fallback_url")
            if (fallback != null && (fallback.startsWith("https://") || fallback.startsWith("http://"))) session.loadUri(fallback) else show(str(R.string.external_none))
        } catch (e: SecurityException) {
            Log.w(TAG, "external app refused", e)
            show(str(R.string.external_none))
        }
    }

    private fun fileName(response: WebResponse): String {
        val disposition = response.headers.entries.firstOrNull { it.key.equals("content-disposition", true) }?.value
        val fromHeader = disposition?.let {
            Regex("filename\\*=(?:UTF-8'')?([^;]+)", RegexOption.IGNORE_CASE).find(it)?.groupValues?.get(1)?.let { v -> Uri.decode(v.trim('"', ' ')) }
                ?: Regex("filename=\"?([^\";]+)", RegexOption.IGNORE_CASE).find(it)?.groupValues?.get(1)
        }
        val raw = fromHeader ?: Uri.parse(response.uri).lastPathSegment ?: "download"
        // No path parts or control characters in a name the page chose.
        return raw.substringAfterLast('/').substringAfterLast('\\').filter { it >= ' ' }.trim().ifEmpty { "download" }.take(120)
    }

    private fun download(response: WebResponse, isPrivate: Boolean) {
        val body = response.body ?: return show(str(R.string.download_failed))
        val name = fileName(response)
        val mime = response.headers.entries.firstOrNull { it.key.equals("content-type", true) }?.value?.substringBefore(';')?.trim()
            ?: "application/octet-stream"
        show(str(R.string.download_started, name))
        Thread {
            val ok = runCatching {
                body.use { input ->
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                        val resolver = context.contentResolver
                        val values = ContentValues().apply {
                            put(MediaStore.Downloads.DISPLAY_NAME, name)
                            put(MediaStore.Downloads.MIME_TYPE, mime)
                            put(MediaStore.Downloads.IS_PENDING, 1)
                        }
                        val item = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values) ?: error("no entry")
                        try {
                            resolver.openOutputStream(item)!!.use { input.copyTo(it) }
                            resolver.update(item, ContentValues().apply { put(MediaStore.Downloads.IS_PENDING, 0) }, null, null)
                        } catch (e: Exception) {
                            resolver.delete(item, null, null)
                            throw e
                        }
                    } else {
                        // Android 8–9: the app's own Downloads folder (no storage permission needed).
                        val dir = context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS) ?: error("no storage")
                        File(dir, name).outputStream().use { input.copyTo(it) }
                    }
                }
            }.onFailure { Log.w(TAG, "download", it) }.isSuccess
            // isPrivate: nothing is recorded about the download beyond the file itself.
            main.post { show(str(if (ok) R.string.download_done else R.string.download_failed, name)) }
        }.start()
    }

    private fun hex(c: androidx.compose.ui.graphics.Color): String = "#%06X".format(c.toArgb() and 0xFFFFFF)

    /** The error page, built from the Aura tokens for both themes (data: URI, no network). */
    private fun errorPage(uri: String): String {
        val host = Uri.parse(uri).host ?: uri
        fun esc(v: String) = v.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;")
        val l = dev.scyde.browser.theme.ScydeColorsLight
        val d = dev.scyde.browser.theme.ScydeColorsDark
        val space = ScydeSpace
        val html = """<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light dark"><title>${esc(str(R.string.loaderror_title))}</title><style>
:root{--bg:${hex(l.surfaceCanvas)};--fg:${hex(l.textPrimary)};--muted:${hex(l.textSecondary)};--btn:${hex(l.textPrimary)};--on:${hex(l.surfaceCanvas)}}
@media (prefers-color-scheme:dark){:root{--bg:${hex(d.surfaceCanvas)};--fg:${hex(d.textPrimary)};--muted:${hex(d.textSecondary)};--btn:${hex(d.textPrimary)};--on:${hex(d.surfaceCanvas)}}}
html,body{margin:0;min-height:100%;background:var(--bg);color:var(--fg);font-family:system-ui,sans-serif}
main{box-sizing:border-box;min-height:100vh;display:flex;flex-direction:column;justify-content:center;padding:${space.s24.value.toInt()}px;max-width:${space.s80.value.toInt() * 6}px;margin:0 auto}
h1{font-size:${dev.scyde.browser.theme.ScydeType.titleSize.value.toInt()}px;margin:0 0 ${space.s8.value.toInt()}px}
p{color:var(--muted);font-size:${dev.scyde.browser.theme.ScydeType.bodySize.value.toInt()}px;line-height:1.45;margin:0 0 ${space.s24.value.toInt()}px}
button{align-self:flex-start;min-height:${space.s48.value.toInt()}px;padding:0 ${space.s24.value.toInt()}px;border:0;border-radius:${dev.scyde.browser.theme.ScydeRadius.full.value.toInt()}px;background:var(--btn);color:var(--on);font:inherit;font-weight:500}
</style></head><body><main><h1>${esc(str(R.string.loaderror_title))}</h1><p>${esc(str(R.string.loaderror_body, host))}</p>
<button onclick="location.reload()">${esc(str(R.string.loaderror_retry))}</button></main></body></html>"""
        return "data:text/html;charset=utf-8;base64," + Base64.encodeToString(html.toByteArray(), Base64.NO_WRAP)
    }

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
        tabs.filter { it.site == site && !it.sleeping }.forEach { it.skipVisit = true; it.session.reload() }
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
                val next = url ?: ""
                // Hash links and history.pushState are not a new page: keep the tint and full screen.
                val samePage = next.substringBefore('#').substringBefore('?') == tab.url.substringBefore('#').substringBefore('?')
                tab.url = next
                if (!samePage) {
                    tab.themeColor = null
                    tab.immersive = false // a new page starts with the island visible
                }
                tab.bookmarked = tab.isWeb && runCatching { places?.isBookmarked(tab.url) }.getOrNull() == true
            }

            /** De-AMP: a Google AMP viewer URL loads the publisher's page, so the island shows the real domain. */
            override fun onLoadRequest(session: GeckoSession, request: GeckoSession.NavigationDelegate.LoadRequest): GeckoResult<AllowOrDeny>? {
                if (isExternal(request.uri)) {
                    // tel:, mailto:, intent:, market:… go to the app that handles them, and only after a tap.
                    if (request.hasUserGesture) main.post { openExternal(request.uri, session) }
                    return GeckoResult.fromValue(AllowOrDeny.DENY)
                }
                val target = omniboxDeamp(request.uri) ?: return null
                main.post { session.loadUri(target) }
                return GeckoResult.fromValue(AllowOrDeny.DENY)
            }

            override fun onCanGoBack(session: GeckoSession, canGoBack: Boolean) {
                tab.canGoBack = canGoBack
            }

            /** A page that did not open: our own error page instead of a blank one (never a TLS bypass). */
            override fun onLoadError(session: GeckoSession, uri: String?, error: WebRequestError): GeckoResult<String>? {
                if (uri == null || !(uri.startsWith("https://") || uri.startsWith("http://"))) return null
                return GeckoResult.fromValue(errorPage(uri))
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

            override fun onSessionStateChange(session: GeckoSession, sessionState: GeckoSession.SessionState) {
                tab.savedState = sessionState
            }

            override fun onPageStop(session: GeckoSession, success: Boolean) {
                tab.progress = 100
                if (!tab.isPrivate) saveSession()
                val url = tab.url
                // Private tabs leave no history.
                val skip = tab.skipVisit
                tab.skipVisit = false
                if (success && !skip && !tab.isPrivate && (url.startsWith("https://") || url.startsWith("http://"))) {
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

            override fun onFullScreen(session: GeckoSession, fullScreen: Boolean) {
                tab.fullscreen = fullScreen
            }

            /** A response the engine does not show (a file): saved to Downloads from the same response. */
            override fun onExternalResponse(session: GeckoSession, response: WebResponse) {
                download(response, tab.isPrivate)
            }

            /** Long-press on a link: Peek (preview and actions). */
            override fun onContextMenu(session: GeckoSession, screenX: Int, screenY: Int, element: GeckoSession.ContentDelegate.ContextElement) {
                val link = element.linkUri ?: return
                if (link.startsWith("http://") || link.startsWith("https://")) openPeek(link, tab.isPrivate)
            }
        }
        s.scrollDelegate = object : GeckoSession.ScrollDelegate {
            override fun onScrollChanged(session: GeckoSession, scrollX: Int, scrollY: Int) {
                // GeckoView reports CSS pixels (about 1 dp on mobile pages), so no density factor.
                tab.scrolled(scrollY, ScydeSpace.s12.value.toInt(), ScydeSpace.s64.value.toInt())
            }
        }
        val prompts = PagePrompts(this) { popupsBlocked++ }
        s.promptDelegate = prompts
        s.permissionDelegate = prompts
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
            val json = message as? JSONObject ?: run {
                Log.w(TAG, "bridge: message is ${message.javaClass.name}, not an object")
                return null
            }
            when (json.optString("type")) {
                "ready" -> {
                    main.post { shieldReady = true; flushPending() }
                    return GeckoResult.fromValue(true)
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
            if (!trusted) {
                Log.w(TAG, "bridge: untrusted sender url=${sender.url} env=${sender.environmentType} base=$base")
                return GeckoResult.fromValue(reply.put("ok", false).put("error", "untrusted origin").toString())
            }
            if (method !in METHODS) {
                Log.w(TAG, "bridge: method not allowed: $method")
                return GeckoResult.fromValue(reply.put("ok", false).put("error", "method not allowed").toString())
            }
            val result = GeckoResult<Any>()
            main.post {
                try {
                    result.complete(reply.put("ok", true).put("result", handle(method, json.optJSONObject("params") ?: JSONObject())).toString())
                } catch (e: Exception) {
                    Log.e(TAG, "bridge: $method failed", e)
                    result.complete(reply.put("ok", false).put("error", e.message ?: "error").toString())
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
        /** Schemes the engine loads itself; any other link goes to the app that handles it. */
        private val INTERNAL_SCHEMES = setOf("http", "https", "about", "data", "blob", "file", "javascript", "moz-extension", "resource", "chrome", "view-source")
        val METHODS = setOf(
            "newtab.state", "navigate", "settings.schema", "settings.get", "settings.set", "settings.reset",
            "history.query", "history.delete", "history.clear", "bookmarks.children", "bookmarks.remove",
            "shield.summary", "shield.setSite",
        )

        @Volatile private var shared: GeckoRuntime? = null

        /**
         * One Browser per process: the GeckoRuntime and the extension (its native port, its `ready`
         * message) are process-wide, so a recreated Activity must reuse the same Browser.
         */
        @Volatile private var instance: Browser? = null

        fun get(context: Context): Browser = instance ?: synchronized(this) { instance ?: Browser(context).also { instance = it } }
    }
}
