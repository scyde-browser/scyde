package dev.scyde.browser

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.graphics.luminance
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import dev.scyde.browser.theme.ScydeAccent
import dev.scyde.browser.theme.ScydeAccents
import dev.scyde.browser.theme.ScydeBrand
import dev.scyde.browser.theme.ScydeColors
import dev.scyde.browser.theme.ScydeColorsDark
import dev.scyde.browser.theme.ScydeColorsLight
import dev.scyde.browser.theme.ScydeMotion
import dev.scyde.browser.theme.ScydeRadius
import dev.scyde.browser.theme.ScydeSize
import dev.scyde.browser.theme.ScydeSpace
import kotlinx.coroutines.launch
import org.mozilla.geckoview.GeckoView
import uniffi.scyde_core.omniboxDisplayUrl
import uniffi.scyde_core.themeIslandTint

class MainActivity : ComponentActivity() {
    private lateinit var browser: Browser
    /** `--es panel menu|shield|omnibox|tabs` opens that surface (CI screenshots). */
    private val initialPanel = mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge() // mandatory for targetSdk 35+ (CLAUDE.md)
        super.onCreate(savedInstanceState)
        browser = Browser.get(this)
        val start = intent?.dataString ?: intent?.getStringExtra("url")
        // Restore the last session (sleeping tabs) unless a link opened the app.
        // Restore the last session first (a link must not overwrite it), then open the link on top.
        if (browser.tabs.isEmpty()) {
            val restored = browser.restoreSession()
            if (start != null) browser.newTab(start) else if (!restored) browser.newTab()
        } else if (start != null && savedInstanceState == null) {
            browser.newTab(start)
        }
        val page = intent?.getStringExtra("page") // CI screenshots
        when {
            page != null -> browser.openPage(page)
            start == null && browser.needsOnboarding && intent?.getBooleanExtra("skipWelcome", false) != true -> browser.openPage("welcome")
        }
        initialPanel.value = intent?.getStringExtra("panel")
        setContent { ScydeApp(browser, initialPanel) }
    }

    override fun onPause() {
        super.onPause()
        browser.saveSession()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        intent.dataString?.let {
            browser.closePeek()
            browser.closeSheet()
            browser.newTab(it)
        }
        intent.getStringExtra("page")?.let { browser.openPage(it) }
        intent.getStringExtra("panel")?.let { initialPanel.value = it }
    }
}

/** What grows out of the island (DESIGN.md §6.3 Swell). */
enum class Panel { NONE, MENU, SHIELD, OMNIBOX, FIND }

private fun accentFor(name: String?, dark: Boolean): ScydeAccent = when (name) {
    "sky" -> if (dark) ScydeAccents.skyDark else ScydeAccents.skyLight
    "periwinkle" -> if (dark) ScydeAccents.periwinkleDark else ScydeAccents.periwinkleLight
    "lavender" -> if (dark) ScydeAccents.lavenderDark else ScydeAccents.lavenderLight
    "mint" -> if (dark) ScydeAccents.mintDark else ScydeAccents.mintLight
    "onyx" -> if (dark) ScydeAccents.onyxDark else ScydeAccents.onyxLight
    else -> if (dark) ScydeAccents.emberDark else ScydeAccents.emberLight
}

private fun <T> smooth() = spring<T>(dampingRatio = ScydeMotion.springSmooth.dampingRatio, stiffness = ScydeMotion.springSmooth.stiffness)

@Composable
fun ScydeApp(browser: Browser, initialPanel: MutableState<String?> = mutableStateOf(null)) {
    val system = isSystemInDarkTheme()
    val dark = when (browser.appearance["theme"]) { "dark" -> true; "light" -> false; else -> system }
    val c = if (dark) ScydeColorsDark else ScydeColorsLight
    val accent = accentFor(browser.appearance["accent"], dark)
    var showTabs by remember { mutableStateOf(false) }
    var panel by remember { mutableStateOf(Panel.NONE) }
    val active = browser.active
    val sheet = browser.sheetPage
    var edgeSide by remember { mutableIntStateOf(0) }
    var edgeFraction by remember { mutableFloatStateOf(0f) }
    val haptic = LocalHapticFeedback.current
    val density = LocalDensity.current

    // Full screen while reading: the system bars hide with the island and come back with a swipe.
    val immersive = active?.immersive == true && panel == Panel.NONE && sheet == null && browser.peekUrl == null
    val view = LocalView.current
    val pageColor = active?.themeColor?.let { runCatching { Color(android.graphics.Color.parseColor(it)) }.getOrNull() }
    val topColor = if (sheet == null) pageColor ?: c.surfaceCanvas else c.surfaceRaised
    LaunchedEffect(immersive, topColor) {
        val window = (view.context as? android.app.Activity)?.window ?: return@LaunchedEffect
        val bars = WindowCompat.getInsetsController(window, view)
        bars.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        bars.isAppearanceLightStatusBars = topColor.luminance() > 0.5f
        if (immersive) bars.hide(WindowInsetsCompat.Type.systemBars()) else bars.show(WindowInsetsCompat.Type.systemBars())
    }

    LaunchedEffect(initialPanel.value) {
        when (initialPanel.value) {
            "menu" -> panel = Panel.MENU
            "shield" -> panel = Panel.SHIELD
            "omnibox" -> panel = Panel.OMNIBOX
            "tabs" -> showTabs = true
            else -> return@LaunchedEffect
        }
        initialPanel.value = null
    }

    fun closePanel() {
        if (panel == Panel.FIND) browser.endFind()
        panel = Panel.NONE
    }

    // Predictive back (CLAUDE.md): peek → sheet → panel → tab grid → page history → exit.
    BackHandler(enabled = browser.peekUrl != null || sheet != null || panel != Panel.NONE || showTabs || active?.canGoBack == true) {
        when {
            browser.peekUrl != null -> browser.closePeek()
            sheet != null -> browser.closeSheet()
            panel != Panel.NONE -> closePanel()
            showTabs -> showTabs = false
            else -> active?.session?.goBack()
        }
    }

    // Behind the status bar: the page's own theme-color, so the page has no band around it.
    Box(Modifier.fillMaxSize().background(c.surfaceCanvas)) {
        Column(Modifier.fillMaxSize().background(topColor)) {
            if (sheet != null) SheetBar(sheet, c, onClose = { browser.closeSheet() })
            AndroidView(
                modifier = Modifier.fillMaxWidth().weight(1f).then(if (sheet == null) Modifier.statusBarsPadding() else Modifier),
                factory = { ctx ->
                    val edge = with(density) { ScydeSpace.s40.toPx() }
                    val distance = with(density) { ScydeSpace.s80.toPx() }
                    EdgeSwipeLayout(
                        ctx, edge, distance,
                        onProgress = { side, f -> edgeSide = side; edgeFraction = f },
                        onCommit = commit@{ side ->
                            if (browser.sheetPage != null) return@commit
                            val tab = browser.active ?: return@commit
                            if (side < 0 && tab.canGoBack) tab.session.goBack()
                            if (side > 0 && tab.canGoForward) tab.session.goForward()
                            haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                        },
                    ).apply { addView(GeckoView(ctx)) }
                },
                update = { frame ->
                    val view = frame.getChildAt(0) as GeckoView
                    val s = browser.sheetSession ?: active?.session
                    if (s != null && view.session != s) {
                        view.releaseSession()
                        view.setSession(s)
                    }
                },
            )
        }
        if (edgeSide != 0 && sheet == null) {
            val can = if (edgeSide < 0) active?.canGoBack == true else active?.canGoForward == true
            EdgeArrow(edgeSide, edgeFraction, can, c, Modifier.align(if (edgeSide < 0) Alignment.CenterStart else Alignment.CenterEnd))
        }
        if (sheet == null) {
            AnimatedVisibility(
                visible = panel != Panel.NONE && panel != Panel.FIND,
                enter = fadeIn(),
                exit = fadeOut(),
                modifier = Modifier.fillMaxSize(),
            ) {
                Box(
                    Modifier.fillMaxSize().background(c.scrim)
                        .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) { closePanel() },
                )
            }
            Column(
                Modifier.align(Alignment.BottomCenter).navigationBarsPadding().imePadding()
                    .padding(horizontal = ScydeSize.islandMobileInset, vertical = ScydeSpace.s8)
                    .widthIn(max = ScydeSize.islandWidth),
                verticalArrangement = Arrangement.spacedBy(ScydeSpace.s8),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                AnimatedVisibility(visible = browser.toast != null && panel == Panel.NONE, enter = fadeIn() + scaleIn(initialScale = 0.96f), exit = fadeOut()) {
                    Toast(browser.toast ?: "", c)
                }
                AnimatedVisibility(
                    visible = panel == Panel.MENU || panel == Panel.SHIELD || panel == Panel.OMNIBOX,
                    enter = fadeIn() + scaleIn(initialScale = 0.96f, transformOrigin = androidx.compose.ui.graphics.TransformOrigin(0.5f, 1f)),
                    exit = fadeOut() + scaleOut(targetScale = 0.96f, transformOrigin = androidx.compose.ui.graphics.TransformOrigin(0.5f, 1f)),
                ) {
                    PanelCard(c) {
                        when (panel) {
                            Panel.MENU -> ControlCenter(browser, c, accent, close = { closePanel() }, onFind = { panel = Panel.FIND })
                            Panel.SHIELD -> ShieldPanel(browser, c, close = { closePanel() })
                            Panel.OMNIBOX -> OmniboxPanel(browser, c, accent, close = { closePanel() }, onFind = { panel = Panel.FIND })
                            else -> Unit
                        }
                    }
                }
                when {
                    panel == Panel.FIND -> FindBar(browser, c, accent, close = { closePanel() })
                    immersive -> RevealStrip(onReveal = { active?.immersive = false })
                    else -> Island(browser, c, accent, dark, panel, setPanel = { panel = it }, onTabs = { showTabs = true })
                }
            }
        }
        if (browser.peekUrl != null) Peek(browser, c, accent)
    }
    if (showTabs) TabGrid(browser, c, accent, onDone = { showTabs = false })
}

/** Aura tint: a little of the page's theme-color, only when the core says island text stays AA. */
private fun islandFill(tab: Tab?, c: ScydeColors, dark: Boolean): Color {
    val tinted = tab?.themeColor?.let { themeIslandTint(it, dark) } ?: return c.islandBg
    return runCatching { Color(android.graphics.Color.parseColor(tinted)) }.getOrDefault(c.islandBg)
}

/** The island (DESIGN.md §8): mark (menu) · back · address · Shield pebble · tabs. */
@Composable
private fun Island(
    browser: Browser,
    c: ScydeColors,
    accent: ScydeAccent,
    dark: Boolean,
    panel: Panel,
    setPanel: (Panel) -> Unit,
    onTabs: () -> Unit,
) {
    val tab = browser.active
    val editing = panel == Panel.OMNIBOX
    var field by remember { mutableStateOf(TextFieldValue("")) }
    var tabMenu by remember { mutableStateOf(false) }
    val focus = remember { FocusRequester() }
    val haptic = LocalHapticFeedback.current
    val shape = RoundedCornerShape(ScydeRadius.island)
    val drag = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()

    fun toggle(p: Panel) = setPanel(if (panel == p) Panel.NONE else p)
    // The gesture block below lives across recompositions: it must read the current tab and mode.
    val currentTab by rememberUpdatedState(tab)
    val currentEditing by rememberUpdatedState(editing)
    val spaced = browser.spaces.size > 1 && tab?.isPrivate != true

    Row(
        Modifier
            .fillMaxWidth()
            .height(ScydeSize.islandMobileHeight)
            .graphicsLayer { translationX = drag.value }
            .shadow(ScydeSpace.s16, shape)
            .background(islandFill(tab, c, dark), shape)
            // With more than one Space, the island's edge takes the Space's colour.
            .border(
                if (spaced) ScydeSpace.s2 * 0.75f else ScydeSpace.s2 / 2,
                if (spaced) accentFor(browser.currentSpace.color, dark).ring else c.islandBorder,
                shape,
            )
            .padding(horizontal = ScydeSpace.s4)
            // Watches in the Initial pass, before the buttons: a drag that starts on a button still
            // switches tabs, and once it is a drag the buttons do not get a click.
            .pointerInput(Unit) {
                val slop = viewConfiguration.touchSlop
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                    var dx = 0f
                    var dy = 0f
                    var dragging = false
                    while (true) {
                        val ev = awaitPointerEvent(PointerEventPass.Initial)
                        val ch = ev.changes.firstOrNull { it.id == down.id } ?: break
                        if (!ch.pressed) break
                        val d = ch.position - ch.previousPosition
                        dx += d.x
                        dy += d.y
                        if (!dragging && !currentEditing && (kotlin.math.abs(dx) > slop || kotlin.math.abs(dy) > slop)) dragging = true
                        if (dragging) {
                            ch.consume()
                            if (kotlin.math.abs(dx) > kotlin.math.abs(dy)) scope.launch { drag.snapTo(dx * 0.35f) }
                        }
                    }
                    scope.launch { drag.animateTo(0f, smooth()) }
                    if (!dragging) return@awaitEachGesture
                    if (dy < -ScydeSpace.s40.toPx() && kotlin.math.abs(dy) > kotlin.math.abs(dx)) {
                        onTabs()
                    } else if (dy > ScydeSpace.s40.toPx() && kotlin.math.abs(dy) > kotlin.math.abs(dx) && currentTab?.isWeb == true) {
                        currentTab?.immersive = true // swipe down: full screen
                    } else if (kotlin.math.abs(dx) > ScydeSpace.s48.toPx()) {
                        browser.switchTab(if (dx < 0) 1 else -1)
                        haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    }
                }
            },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s2),
    ) {
        if (editing) {
            LaunchedEffect(Unit) { focus.requestFocus() }
            BasicTextField(
                value = field,
                onValueChange = { v ->
                    val grew = v.text.length > field.text.length && v.selection.collapsed
                    field = v
                    // The panel searches what was typed, not the inline completion.
                    OmniboxQuery.text = v.text
                    if (grew) browser.suggestion(v.text)?.let { rest ->
                        field = TextFieldValue(v.text + rest, TextRange(v.text.length, v.text.length + rest.length))
                    }
                },
                singleLine = true,
                textStyle = TextStyle(color = c.islandText, fontSize = 16.sp),
                cursorBrush = SolidColor(accent.fill),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go, autoCorrectEnabled = false),
                keyboardActions = KeyboardActions(onGo = { browser.go(field.text); setPanel(Panel.NONE) }),
                modifier = Modifier.weight(1f).focusRequester(focus)
                    .background(c.islandRaised, CircleShape).padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s12)
                    .semantics { contentDescription = "address" },
            )
            Text(
                stringResource(R.string.omnibox_cancel), color = c.islandText, fontSize = 15.sp,
                modifier = Modifier.clickable { setPanel(Panel.NONE) }.padding(horizontal = ScydeSpace.s12, vertical = ScydeSpace.s12),
            )
        } else {
            IslandButton(stringResource(R.string.island_menu), onClick = { toggle(Panel.MENU) }) {
                if (tab?.isPrivate == true) {
                    Icon(ScydeIcons.incognito, null, tint = c.islandText, modifier = Modifier.size(ScydeSpace.s24))
                } else {
                    Canvas(Modifier.size(ScydeSpace.s24)) { drawPath(brandMarkPath(size.minDimension), ScydeBrand.mark) }
                }
            }
            AnimatedVisibility(visible = tab?.canGoBack == true, enter = fadeIn() + scaleIn(initialScale = 0.96f), exit = fadeOut() + scaleOut(targetScale = 0.96f)) {
                IslandButton(stringResource(R.string.nav_back), onClick = { tab?.session?.goBack() }) {
                    Icon(ScydeIcons.back, null, tint = c.islandText, modifier = Modifier.size(ScydeSpace.s20 + ScydeSpace.s2))
                }
            }
            Box(Modifier.weight(1f).height(ScydeSize.hitAndroid), contentAlignment = Alignment.Center) {
                val page = tab?.url?.let { browser.internalPage(it) }
                val display = if (page == null && tab != null && tab.isWeb) omniboxDisplayUrl(tab.url) else null
                val label = when {
                    page == "newtab" || tab == null || tab.url.isEmpty() -> null
                    page != null -> "scyde://$page"
                    display != null -> display.domain
                    else -> tab.url
                }
                Row(
                    Modifier.fillMaxSize().clickable {
                        val u = tab?.url ?: ""
                        val text = if (page != null) (if (page == "newtab") "" else "scyde://$page") else u
                        field = TextFieldValue(text, TextRange(0, text.length))
                        OmniboxQuery.text = "" // the whole address is selected: show recent pages and actions
                        setPanel(Panel.OMNIBOX)
                    },
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.Center,
                ) {
                    if (tab?.isPrivate == true) {
                        Text(
                            stringResource(R.string.private_label), color = c.islandText, fontSize = 11.sp, fontFamily = FontFamily.Monospace,
                            modifier = Modifier.background(c.islandRaised, CircleShape).padding(horizontal = ScydeSpace.s6, vertical = ScydeSpace.s2),
                        )
                        Spacer(Modifier.width(ScydeSpace.s6))
                    }
                    if (display?.scheme == "https") {
                        Icon(ScydeIcons.lock, null, tint = c.islandTextSecondary, modifier = Modifier.size(ScydeSpace.s16))
                        Spacer(Modifier.width(ScydeSpace.s4))
                    }
                    Text(
                        text = label ?: stringResource(R.string.omnibox_placeholder),
                        color = if (label == null) c.islandTextSecondary else c.islandText,
                        fontSize = 16.sp,
                        maxLines = 1,
                        overflow = TextOverflow.StartEllipsis,
                    )
                }
                val p = tab?.progress ?: 100
                if (p in 1..99) {
                    Box(Modifier.align(Alignment.BottomStart).fillMaxWidth(p / 100f).height(ScydeSpace.s2).background(accent.fill, CircleShape))
                }
            }
            val on = browser.shieldOn(tab)
            val count = if (on) browser.blockedOn(tab) else null
            Row(
                Modifier.height(ScydeSpace.s40)
                    .background(if (panel == Panel.SHIELD) c.islandRaised else c.islandRaised.copy(alpha = 0.6f), CircleShape)
                    .clickable { toggle(Panel.SHIELD) }
                    .padding(horizontal = ScydeSpace.s12)
                    .semantics { contentDescription = "Shield" },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s6),
            ) {
                Box(Modifier.size(ScydeSpace.s8).background(if (on && browser.shieldReady) c.islandShield else c.islandTextSecondary, CircleShape))
                if (count != null && count > 0) Text("$count", color = c.islandText, fontSize = 13.sp, fontFamily = FontFamily.Monospace)
            }
            Box {
                val n = browser.visibleTabs(tab?.isPrivate ?: false).size
                Box(
                    Modifier.size(ScydeSize.hitAndroid)
                        .combinedClickable(onClick = onTabs, onLongClick = { tabMenu = true })
                        .semantics { contentDescription = "tabs" },
                    contentAlignment = Alignment.Center,
                ) {
                    Box(
                        Modifier.size(ScydeSpace.s24 + ScydeSpace.s2).border(ScydeSpace.s2 * 0.75f, c.islandText, RoundedCornerShape(ScydeRadius.xs)),
                        contentAlignment = Alignment.Center,
                    ) { Text("$n", color = c.islandText, fontSize = 12.sp, fontFamily = FontFamily.Monospace) }
                }
                DropdownMenu(expanded = tabMenu, onDismissRequest = { tabMenu = false }) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.island_new_tab)) }, onClick = { tabMenu = false; browser.newTab() })
                    DropdownMenuItem(text = { Text(stringResource(R.string.menu_new_private_tab)) }, onClick = { tabMenu = false; browser.newTab(isPrivate = true) })
                    if (tab != null) DropdownMenuItem(text = { Text(stringResource(R.string.tab_close)) }, onClick = { tabMenu = false; browser.close(tab) })
                }
            }
        }
    }
}

/** The text the omnibox panel searches for (typed part only, without the inline completion). */
private object OmniboxQuery {
    var text by mutableStateOf("")
}

/** While the island is hidden: a strip at the bottom edge that brings it back (tap or swipe up). */
@Composable
private fun RevealStrip(onReveal: () -> Unit) {
    val label = stringResource(R.string.island_show)
    Box(
        Modifier.fillMaxWidth().height(ScydeSpace.s24)
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onReveal)
            .pointerInput(Unit) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    var dy = 0f
                    while (true) {
                        val ev = awaitPointerEvent()
                        val ch = ev.changes.firstOrNull { it.id == down.id } ?: break
                        if (!ch.pressed) break
                        dy += (ch.position - ch.previousPosition).y
                    }
                    if (dy < -ScydeSpace.s12.toPx()) onReveal()
                }
            }
            .semantics { contentDescription = label },
    )
}

@Composable
private fun PanelCard(c: ScydeColors, content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(ScydeRadius.xl)
    Box(Modifier.fillMaxWidth().shadow(ScydeSpace.s16, shape).background(c.surfaceRaised, shape)) { content() }
}

/** Every page and browser action in one place (the island's menu). */
@Composable
private fun ControlCenter(browser: Browser, c: ScydeColors, accent: ScydeAccent, close: () -> Unit, onFind: () -> Unit) {
    val tab = browser.active
    val web = tab?.isWeb == true
    Column(Modifier.padding(ScydeSpace.s12), verticalArrangement = Arrangement.spacedBy(ScydeSpace.s8)) {
        Row(horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s8)) {
            NavTile(ScydeIcons.back, stringResource(R.string.nav_back), tab?.canGoBack == true, c, Modifier.weight(1f)) { tab?.session?.goBack(); close() }
            NavTile(ScydeIcons.forward, stringResource(R.string.nav_forward), tab?.canGoForward == true, c, Modifier.weight(1f)) { tab?.session?.goForward(); close() }
            val loading = (tab?.progress ?: 100) in 1..99
            NavTile(if (loading) ScydeIcons.close else ScydeIcons.reload, stringResource(if (loading) R.string.nav_stop else R.string.nav_reload), tab != null, c, Modifier.weight(1f)) {
                browser.reloadOrStop(); close()
            }
            NavTile(ScydeIcons.share, stringResource(R.string.mobile_share), web, c, Modifier.weight(1f)) { browser.share(); close() }
        }
        val tiles: List<@Composable (Modifier) -> Unit> = listOf(
            { m -> ActionTile(ScydeIcons.plus, stringResource(R.string.island_new_tab), c, accent, m) { browser.newTab(); close() } },
            { m -> ActionTile(ScydeIcons.incognito, stringResource(R.string.menu_new_private_tab), c, accent, m) { browser.newTab(isPrivate = true); close() } },
            { m ->
                val on = tab?.bookmarked == true
                ActionTile(if (on) ScydeIcons.starFilled else ScydeIcons.star, stringResource(if (on) R.string.menu_bookmark_remove else R.string.menu_bookmark_add), c, accent, m, on = on, enabled = web) {
                    browser.toggleBookmark(); close()
                }
            },
            { m -> ActionTile(ScydeIcons.find, stringResource(R.string.menu_find), c, accent, m, enabled = web) { onFind() } },
            { m -> ActionTile(ScydeIcons.desktop, stringResource(R.string.menu_desktop_site), c, accent, m, on = tab?.desktop == true, enabled = web) { browser.toggleDesktop(); close() } },
            { m -> ActionTile(ScydeIcons.copy, stringResource(R.string.menu_copy_link), c, accent, m, enabled = web) { browser.copyLink(); close() } },
            { m -> ActionTile(ScydeIcons.history, stringResource(R.string.library_tab_history), c, accent, m) { browser.openPage("library#history"); close() } },
            { m -> ActionTile(ScydeIcons.book, stringResource(R.string.library_tab_bookmarks), c, accent, m) { browser.openPage("library#bookmarks"); close() } },
            { m -> ActionTile(ScydeIcons.settings, stringResource(R.string.rail_settings), c, accent, m) { browser.openPage("settings"); close() } },
        )
        tiles.chunked(3).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s8)) { row.forEach { it(Modifier.weight(1f)) } }
        }
    }
}

@Composable
private fun NavTile(icon: ImageVector, label: String, enabled: Boolean, c: ScydeColors, modifier: Modifier, onClick: () -> Unit) {
    Box(
        modifier.height(ScydeSize.hitAndroid).background(c.surfaceSunken, RoundedCornerShape(ScydeRadius.lg))
            .clickable(enabled = enabled, onClick = onClick).semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) { Icon(icon, null, tint = if (enabled) c.textPrimary else c.textTertiary, modifier = Modifier.size(ScydeSpace.s24)) }
}

@Composable
private fun ActionTile(
    icon: ImageVector,
    label: String,
    c: ScydeColors,
    accent: ScydeAccent,
    modifier: Modifier,
    on: Boolean = false,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    val fg = when {
        on -> accent.onFill
        enabled -> c.textPrimary
        else -> c.textTertiary
    }
    Column(
        modifier.heightIn(min = ScydeSpace.s80).background(if (on) accent.fill else c.surfaceSunken, RoundedCornerShape(ScydeRadius.lg))
            .clickable(enabled = enabled, onClick = onClick).padding(ScydeSpace.s12),
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Icon(icon, null, tint = if (on) accent.onFill else if (enabled) c.textSecondary else c.textTertiary, modifier = Modifier.size(ScydeSpace.s24))
        Text(label, color = fg, fontSize = 13.sp, lineHeight = 16.sp, maxLines = 2)
    }
}

/** Shield for the current site: one switch, the count, and the way to the details. */
@Composable
private fun ShieldPanel(browser: Browser, c: ScydeColors, close: () -> Unit) {
    val tab = browser.active
    val site = tab?.site
    val on = browser.shieldOn(tab)
    Column(Modifier.padding(ScydeSpace.s20), verticalArrangement = Arrangement.spacedBy(ScydeSpace.s16)) {
        Box(Modifier.align(Alignment.CenterHorizontally).width(ScydeSpace.s40).height(ScydeSpace.s4).background(c.borderStrong, CircleShape))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(if (site != null) stringResource(R.string.shield_panel_title, site) else stringResource(R.string.shield_panel_on), color = c.textPrimary, fontSize = 20.sp, fontWeight = FontWeight.Medium)
                Text(stringResource(if (on) R.string.shield_panel_on else R.string.shield_panel_off), color = c.textSecondary, fontSize = 13.sp, fontFamily = FontFamily.Monospace)
            }
            if (site != null) {
                Switch(
                    checked = on,
                    onCheckedChange = { browser.setShield(site, it) },
                    colors = SwitchDefaults.colors(checkedTrackColor = c.statusShield),
                    modifier = Modifier.semantics { contentDescription = "Shield" },
                )
            }
        }
        Row(Modifier.fillMaxWidth().border(ScydeSpace.s2 / 2, c.borderHairline, RoundedCornerShape(ScydeRadius.md))) {
            Stat("${browser.blockedOn(tab) ?: 0}", stringResource(R.string.shield_panel_blocked), c, Modifier.weight(1f))
            Stat("${browser.blocked + browser.popupsBlocked}", stringResource(R.string.shield_panel_today), c, Modifier.weight(1f))
        }
        Row(
            Modifier.fillMaxWidth().height(ScydeSize.hitAndroid).clickable { browser.openPage("shields"); close() },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(stringResource(R.string.shield_panel_open), color = c.textPrimary, fontSize = 16.sp, modifier = Modifier.weight(1f))
            Icon(ScydeIcons.chevron, null, tint = c.textTertiary, modifier = Modifier.size(ScydeSpace.s20))
        }
    }
}

@Composable
private fun Stat(value: String, label: String, c: ScydeColors, modifier: Modifier) {
    Column(modifier.padding(ScydeSpace.s12)) {
        Text(value, color = c.textPrimary, fontSize = 28.sp, fontWeight = FontWeight.Medium)
        Text(label, color = c.textSecondary, fontSize = 12.sp, fontFamily = FontFamily.Monospace)
    }
}

/** The omnibox panel (DESIGN.md §8 Suggestions), also the command bar: tabs, history, search, actions. */
@Composable
private fun OmniboxPanel(browser: Browser, c: ScydeColors, accent: ScydeAccent, close: () -> Unit, onFind: () -> Unit) {
    val q = OmniboxQuery.text.trim()
    val needle = q.lowercase()
    val tabs = if (q.isEmpty()) emptyList() else browser.tabs.filter {
        it.id != browser.activeId && (it.title.lowercase().contains(needle) || it.url.lowercase().contains(needle))
    }.take(3)
    val suggestions = remember(q) {
        runCatching {
            if (q.isEmpty()) browser.places?.history(null, 5u, 0u)?.map { Triple(it.url, it.title, false) }
            else browser.places?.suggest(q, 5u)?.map { Triple(it.url, it.title, it.bookmarked) }
        }.getOrNull() ?: emptyList()
    }
    data class Action(val label: String, val icon: ImageVector, val run: () -> Unit)
    val actions = listOf(
        Action(stringResource(R.string.menu_new_private_tab), ScydeIcons.incognito) { browser.newTab(isPrivate = true) },
        Action(stringResource(R.string.island_new_tab), ScydeIcons.plus) { browser.newTab() },
        Action(stringResource(R.string.menu_find), ScydeIcons.find) { onFind() },
        Action(stringResource(R.string.menu_desktop_site), ScydeIcons.desktop) { browser.toggleDesktop() },
        Action(stringResource(R.string.menu_bookmark_add), ScydeIcons.star) { browser.toggleBookmark() },
        Action(stringResource(R.string.library_tab_history), ScydeIcons.history) { browser.openPage("library#history") },
        Action(stringResource(R.string.library_tab_bookmarks), ScydeIcons.book) { browser.openPage("library#bookmarks") },
        Action(stringResource(R.string.shield_label), ScydeIcons.shield) { browser.openPage("shields") },
        Action(stringResource(R.string.rail_settings), ScydeIcons.settings) { browser.openPage("settings") },
    ).let { all -> if (q.isEmpty()) all.take(4) else all.filter { it.label.lowercase().contains(needle) } }
    val maxHeight = (LocalConfiguration.current.screenHeightDp * 0.5f)
    Column(Modifier.heightIn(max = maxHeight.dp).verticalScroll(rememberScrollState()).padding(vertical = ScydeSpace.s8)) {
        if (tabs.isNotEmpty()) {
            Header(stringResource(R.string.omnibox_section_tabs), c)
            tabs.forEach { t ->
                SuggestionRow(ScydeIcons.tabs, t.title.ifEmpty { t.url }, t.site ?: "", stringResource(R.string.omnibox_switch_to_tab), c, accent) {
                    browser.activeId = t.id; close()
                }
            }
        }
        if (suggestions.isNotEmpty() || q.isNotEmpty()) {
            Header(stringResource(R.string.omnibox_section_suggestions), c)
            suggestions.forEach { (url, title, bookmarked) ->
                SuggestionRow(
                    if (bookmarked) ScydeIcons.star else ScydeIcons.history, title.ifEmpty { url }, omniboxDisplayUrl(url)?.domain ?: url,
                    stringResource(if (bookmarked) R.string.omnibox_suggest_bookmark else R.string.omnibox_suggest_history), c, accent,
                ) { browser.go(url); close() }
            }
            if (q.isNotEmpty()) {
                SuggestionRow(ScydeIcons.search, stringResource(R.string.omnibox_suggest_search, q), "", "", c, accent) { browser.go(q); close() }
            }
        }
        if (actions.isNotEmpty()) {
            Header(stringResource(R.string.omnibox_section_actions), c)
            actions.chunked(2).forEach { row ->
                Row(Modifier.padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s4), horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s8)) {
                    row.forEach { a ->
                        Row(
                            Modifier.weight(1f).height(ScydeSize.hitAndroid).background(c.surfaceSunken, CircleShape)
                                .clickable { a.run(); if (a.icon != ScydeIcons.find) close() }.padding(horizontal = ScydeSpace.s12),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s6),
                        ) {
                            Icon(a.icon, null, tint = c.textSecondary, modifier = Modifier.size(ScydeSpace.s16 + ScydeSpace.s2))
                            Text(a.label, color = c.textPrimary, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        }
    }
}

@Composable
private fun Header(text: String, c: ScydeColors) {
    Text(text, color = c.textTertiary, fontSize = 11.sp, fontFamily = FontFamily.Monospace, modifier = Modifier.padding(start = ScydeSpace.s16, top = ScydeSpace.s8, bottom = ScydeSpace.s4))
}

@Composable
private fun SuggestionRow(icon: ImageVector, title: String, sub: String, source: String, c: ScydeColors, accent: ScydeAccent, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = ScydeSize.hitAndroid + ScydeSpace.s4).clickable(onClick = onClick).padding(horizontal = ScydeSpace.s16),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s12),
    ) {
        Icon(icon, null, tint = c.textSecondary, modifier = Modifier.size(ScydeSpace.s20))
        Column(Modifier.weight(1f)) {
            Text(title, color = c.textPrimary, fontSize = 15.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (sub.isNotEmpty()) Text(sub, color = accent.ink, fontSize = 12.sp, maxLines = 1)
        }
        if (source.isNotEmpty()) Text(source, color = c.textTertiary, fontSize = 11.sp, fontFamily = FontFamily.Monospace, maxLines = 1)
    }
}

/** Find in page, in place of the island. */
@Composable
private fun FindBar(browser: Browser, c: ScydeColors, accent: ScydeAccent, close: () -> Unit) {
    var text by remember { mutableStateOf("") }
    val focus = remember { FocusRequester() }
    val shape = RoundedCornerShape(ScydeRadius.island)
    LaunchedEffect(Unit) { focus.requestFocus() }
    Row(
        Modifier.fillMaxWidth().height(ScydeSize.islandMobileHeight).shadow(ScydeSpace.s16, shape).background(c.islandBg, shape)
            .border(ScydeSpace.s2 / 2, c.islandBorder, shape).padding(horizontal = ScydeSpace.s8),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        BasicTextField(
            value = text,
            onValueChange = { text = it; browser.find(it) },
            singleLine = true,
            textStyle = TextStyle(color = c.islandText, fontSize = 16.sp),
            cursorBrush = SolidColor(accent.fill),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { browser.find(null) }),
            modifier = Modifier.weight(1f).focusRequester(focus).background(c.islandRaised, CircleShape)
                .padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s12).semantics { contentDescription = "find" },
        )
        val r = browser.findResult
        Text(
            when {
                r == null -> ""
                r.second == 0 -> stringResource(R.string.find_none)
                else -> stringResource(R.string.find_count, "${r.first}", "${r.second}")
            },
            color = c.islandTextSecondary, fontSize = 12.sp, fontFamily = FontFamily.Monospace, modifier = Modifier.padding(horizontal = ScydeSpace.s8),
        )
        IslandButton(stringResource(R.string.find_previous), onClick = { browser.find(null, backwards = true) }) {
            Icon(ScydeIcons.up, null, tint = c.islandText, modifier = Modifier.size(ScydeSpace.s20))
        }
        IslandButton(stringResource(R.string.find_next), onClick = { browser.find(null) }) {
            Icon(ScydeIcons.down, null, tint = c.islandText, modifier = Modifier.size(ScydeSpace.s20))
        }
        IslandButton(stringResource(R.string.find_close), onClick = close) {
            Icon(ScydeIcons.close, null, tint = c.islandText, modifier = Modifier.size(ScydeSpace.s20))
        }
    }
}

/** Peek: a long-pressed link in a live preview, with what to do with it. */
@Composable
private fun Peek(browser: Browser, c: ScydeColors, accent: ScydeAccent) {
    val url = browser.peekUrl ?: return
    val session = browser.peekSession
    val isPrivate = browser.active?.isPrivate ?: false
    val shape = RoundedCornerShape(ScydeRadius.xl)
    Box(
        Modifier.fillMaxSize().background(c.scrim)
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) { browser.closePeek() }
            .windowInsetsPadding(WindowInsets.safeDrawing).padding(ScydeSpace.s16),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            Modifier.fillMaxWidth().shadow(ScydeSpace.s16, shape).background(c.surfaceRaised, shape)
                // Taps on the card stay on the card (the scrim around it closes Peek).
                .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {},
        ) {
            Text(
                omniboxDisplayUrl(url)?.domain ?: url, color = accent.ink, fontSize = 13.sp, fontFamily = FontFamily.Monospace, maxLines = 1,
                modifier = Modifier.padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s12),
            )
            if (session != null) {
                AndroidView(
                    factory = { ctx -> GeckoView(ctx).apply { setViewBackend(GeckoView.BACKEND_TEXTURE_VIEW) } },
                    update = { v -> if (v.session != session) { v.releaseSession(); v.setSession(session) } },
                    onRelease = { v -> v.releaseSession() },
                    modifier = Modifier.fillMaxWidth().fillMaxHeight(0.6f),
                )
            }
            Column(Modifier.padding(ScydeSpace.s8)) {
                PeekAction(ScydeIcons.open, stringResource(R.string.peek_open), c) { browser.closePeek(); browser.active?.session?.loadUri(url) }
                PeekAction(ScydeIcons.plus, stringResource(R.string.peek_new_tab), c) { browser.closePeek(); browser.newTab(url, isPrivate = isPrivate) }
                PeekAction(ScydeIcons.tabs, stringResource(R.string.peek_background), c) {
                    browser.closePeek(); browser.newTab(url, activate = false, isPrivate = isPrivate)
                }
                PeekAction(ScydeIcons.incognito, stringResource(R.string.peek_private), c) { browser.closePeek(); browser.newTab(url, isPrivate = true) }
                PeekAction(ScydeIcons.copy, stringResource(R.string.peek_copy), c) { browser.copyLink(url); browser.closePeek() }
                PeekAction(ScydeIcons.share, stringResource(R.string.peek_share), c) { browser.share(url); browser.closePeek() }
            }
        }
    }
}

@Composable
private fun PeekAction(icon: ImageVector, label: String, c: ScydeColors, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().height(ScydeSize.hitAndroid).clickable(onClick = onClick).padding(horizontal = ScydeSpace.s12),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s12),
    ) {
        Icon(icon, null, tint = c.textSecondary, modifier = Modifier.size(ScydeSpace.s20))
        Text(label, color = c.textPrimary, fontSize = 15.sp)
    }
}

@Composable
private fun Toast(message: String, c: ScydeColors) {
    Text(
        message, color = c.islandText, fontSize = 13.sp,
        modifier = Modifier.background(c.islandBg, CircleShape).border(ScydeSpace.s2 / 2, c.islandBorder, CircleShape)
            .padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s12),
    )
}

/** Title and × for Settings, Library and Shield (they are sheets, not tabs). */
@Composable
private fun SheetBar(page: String, c: ScydeColors, onClose: () -> Unit) {
    val title = when (page) {
        "library" -> stringResource(R.string.library_title)
        "shields" -> stringResource(R.string.shield_label)
        "welcome" -> stringResource(R.string.welcome_title)
        else -> stringResource(R.string.rail_settings)
    }
    Row(
        Modifier.fillMaxWidth().background(c.surfaceRaised).statusBarsPadding().height(ScydeSize.hitAndroid + ScydeSpace.s8)
            .padding(horizontal = ScydeSpace.s8),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, color = c.textPrimary, fontSize = 18.sp, modifier = Modifier.weight(1f).padding(horizontal = ScydeSpace.s12))
        IslandButton(stringResource(R.string.sheet_close), onClick = onClose) {
            Icon(ScydeIcons.close, null, tint = c.textPrimary, modifier = Modifier.size(ScydeSpace.s24))
        }
    }
}

/** The back/forward arrow that follows an edge swipe; full opacity means "release to go". */
@Composable
private fun EdgeArrow(side: Int, fraction: Float, enabled: Boolean, c: ScydeColors, modifier: Modifier) {
    Box(
        modifier
            .graphicsLayer {
                translationX = -side * fraction * ScydeSpace.s48.toPx()
                alpha = if (enabled) fraction else fraction * 0.4f
                scaleX = 0.6f + 0.4f * fraction
                scaleY = 0.6f + 0.4f * fraction
            }
            .size(ScydeSize.hitAndroid)
            .shadow(ScydeSpace.s8, CircleShape)
            .background(c.islandBg, CircleShape),
        contentAlignment = Alignment.Center,
    ) { Icon(if (side < 0) ScydeIcons.back else ScydeIcons.forward, null, tint = c.islandText, modifier = Modifier.size(ScydeSpace.s24)) }
}

@Composable
private fun IslandButton(label: String, enabled: Boolean = true, onClick: () -> Unit, content: @Composable () -> Unit) {
    Box(
        Modifier.size(ScydeSize.hitAndroid).clickable(enabled = enabled, onClick = onClick).semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) { content() }
}

/** All tabs as a grid (a full-screen shelf): Spaces on top, pinned first, private apart. */
@Composable
private fun TabGrid(browser: Browser, c: ScydeColors, accent: ScydeAccent, onDone: () -> Unit) {
    var showPrivate by remember { mutableStateOf(browser.active?.isPrivate ?: false) }
    var editing by remember { mutableStateOf<Space?>(null) }
    var creating by remember { mutableStateOf(false) }
    val dark = isSystemInDarkTheme().let { sys -> when (browser.appearance["theme"]) { "dark" -> true; "light" -> false; else -> sys } }
    val list = browser.visibleTabs(showPrivate)
    Dialog(onDismissRequest = onDone, properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Column(Modifier.fillMaxSize().background(c.surfaceCanvas).windowInsetsPadding(WindowInsets.safeDrawing)) {
            Row(Modifier.fillMaxWidth().padding(ScydeSpace.s8), verticalAlignment = Alignment.CenterVertically) {
                val closable = list.any { !it.pinned }
                Text(
                    stringResource(R.string.mobile_tabs_close_all), color = if (closable) c.statusDanger else c.textTertiary,
                    modifier = Modifier.clickable(enabled = closable) { browser.closeAll(showPrivate) }.padding(ScydeSpace.s12),
                )
                Row(Modifier.weight(1f), horizontalArrangement = Arrangement.Center) {
                    Segment(stringResource(R.string.mobile_tabs), !showPrivate, c) { showPrivate = false }
                    Segment(stringResource(R.string.private_label), showPrivate, c) { showPrivate = true }
                }
                Text(stringResource(R.string.mobile_tabs_done), color = accent.ink, fontWeight = FontWeight.Medium, modifier = Modifier.clickable(onClick = onDone).padding(ScydeSpace.s12))
            }
            if (!showPrivate) {
                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s4),
                    horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s8),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    browser.spaces.forEach { space ->
                        val on = space.id == browser.spaceId
                        val tint = accentFor(space.color, dark)
                        Row(
                            Modifier.height(ScydeSize.hitAndroid)
                                .background(if (on) c.surfaceRaised else c.surfaceSunken, CircleShape)
                                .border(ScydeSpace.s2 * 0.75f, if (on) tint.ring else Color.Transparent, CircleShape)
                                .combinedClickable(onClick = { browser.switchSpace(space.id) }, onLongClick = { editing = space })
                                .padding(horizontal = ScydeSpace.s12),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s6),
                        ) {
                            Box(Modifier.size(ScydeSpace.s12).background(tint.fill, CircleShape))
                            Text(space.name, color = c.textPrimary, fontSize = 14.sp, fontWeight = if (on) FontWeight.Medium else FontWeight.Normal, maxLines = 1)
                        }
                    }
                    Box(
                        Modifier.size(ScydeSize.hitAndroid).background(c.surfaceSunken, CircleShape).clickable { creating = true }
                            .semantics { contentDescription = "new space" },
                        contentAlignment = Alignment.Center,
                    ) { Icon(ScydeIcons.plus, null, tint = c.textPrimary, modifier = Modifier.size(ScydeSpace.s20)) }
                }
            }
            if (list.isEmpty()) {
                Text(stringResource(R.string.private_hint), color = c.textSecondary, fontSize = 14.sp, modifier = Modifier.padding(ScydeSpace.s40))
            }
            LazyVerticalGrid(
                columns = GridCells.Adaptive(ScydeSpace.s80 * 2), modifier = Modifier.weight(1f),
                contentPadding = androidx.compose.foundation.layout.PaddingValues(ScydeSpace.s16),
                horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s12), verticalArrangement = Arrangement.spacedBy(ScydeSpace.s12),
            ) {
                items(list, key = { it.id }) { tab -> TabCard(browser, tab, c, accent, onOpen = { browser.activeId = tab.id; onDone() }) }
            }
            Row(
                Modifier.fillMaxWidth().padding(ScydeSpace.s12).height(ScydeSize.hitAndroid).background(c.surfaceRaised, CircleShape)
                    .clickable { browser.newTab(isPrivate = showPrivate); onDone() },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.Center,
            ) {
                Icon(if (showPrivate) ScydeIcons.incognito else ScydeIcons.plus, null, tint = c.textPrimary, modifier = Modifier.size(ScydeSpace.s20))
                Spacer(Modifier.width(ScydeSpace.s8))
                Text(stringResource(if (showPrivate) R.string.menu_new_private_tab else R.string.island_new_tab), color = c.textPrimary, fontSize = 15.sp)
            }
        }
        if (creating) {
            SpaceNameDialog(stringResource(R.string.space_new), "", c, onDone = { name -> creating = false; if (name != null) browser.addSpace(name) })
        }
        editing?.let { space ->
            SpaceEditDialog(browser, space, c, dark, onDone = { editing = null })
        }
    }
}

@Composable
private fun TabCard(browser: Browser, tab: Tab, c: ScydeColors, accent: ScydeAccent, onOpen: () -> Unit) {
    val on = tab.id == browser.activeId
    var menu by remember { mutableStateOf(false) }
    Box {
        Column(
            Modifier.background(c.surfaceRaised, RoundedCornerShape(ScydeRadius.md))
                .border(ScydeSpace.s2, if (on) accent.ring else c.surfaceRaised, RoundedCornerShape(ScydeRadius.md))
                .combinedClickable(onClick = onOpen, onLongClick = { menu = true }).padding(ScydeSpace.s8),
        ) {
            Box(
                Modifier.fillMaxWidth().aspectRatio(0.75f).graphicsLayer { alpha = if (tab.sleeping) ScydeMotion.sleepOpacity else 1f }
                    .background(c.surfaceSunken, RoundedCornerShape(ScydeRadius.sm)),
                contentAlignment = Alignment.Center,
            ) {
                Text(tab.site ?: tab.url.substringAfter("://").substringBefore('/'), color = c.textTertiary, fontSize = 11.sp, maxLines = 1, fontFamily = FontFamily.Monospace)
                if (tab.sleeping) {
                    Text(
                        stringResource(R.string.tab_sleeping_short), color = c.textSecondary, fontSize = 11.sp,
                        modifier = Modifier.align(Alignment.BottomStart).padding(ScydeSpace.s8),
                    )
                }
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (tab.pinned) {
                    Text("•", color = accent.ink, fontSize = 16.sp, modifier = Modifier.padding(top = ScydeSpace.s8, end = ScydeSpace.s4).semantics { contentDescription = "pinned" })
                }
                Text(tab.title.ifEmpty { stringResource(R.string.tab_new) }, color = c.textPrimary, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f).padding(top = ScydeSpace.s8))
                Box(Modifier.size(ScydeSize.hitAndroid).clickable { browser.close(tab) }.semantics { contentDescription = "close tab" }, contentAlignment = Alignment.Center) {
                    Icon(ScydeIcons.close, null, tint = c.textSecondary, modifier = Modifier.size(ScydeSpace.s16))
                }
            }
        }
        DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
            if (!tab.isPrivate) {
                DropdownMenuItem(
                    text = { Text(stringResource(if (tab.pinned) R.string.tab_unpin else R.string.tab_pin)) },
                    onClick = { menu = false; browser.togglePin(tab) },
                )
                browser.spaces.filter { it.id != tab.spaceId }.forEach { space ->
                    DropdownMenuItem(
                        text = { Text("${stringResource(R.string.space_move)}: ${space.name}") },
                        onClick = { menu = false; browser.move(tab, space.id) },
                    )
                }
            }
            DropdownMenuItem(text = { Text(stringResource(R.string.tab_close)) }, onClick = { menu = false; browser.close(tab) })
        }
    }
}

/** A name for a new Space (or a renamed one); `null` when cancelled. */
@Composable
private fun SpaceNameDialog(title: String, initial: String, c: ScydeColors, onDone: (String?) -> Unit) {
    var name by remember { mutableStateOf(initial) }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = { onDone(null) },
        title = { Text(title) },
        text = {
            androidx.compose.material3.OutlinedTextField(
                value = name, onValueChange = { name = it }, singleLine = true,
                label = { Text(stringResource(R.string.space_name)) },
            )
        },
        confirmButton = { androidx.compose.material3.TextButton(onClick = { onDone(name) }) { Text(stringResource(R.string.space_create)) } },
        dismissButton = { androidx.compose.material3.TextButton(onClick = { onDone(null) }) { Text(stringResource(R.string.omnibox_cancel)) } },
        containerColor = c.surfaceRaised,
    )
}

/** Rename, recolour or delete a Space. */
@Composable
private fun SpaceEditDialog(browser: Browser, space: Space, c: ScydeColors, dark: Boolean, onDone: () -> Unit) {
    var name by remember { mutableStateOf(space.name) }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDone,
        title = { Text(stringResource(R.string.space_rename)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(ScydeSpace.s16)) {
                androidx.compose.material3.OutlinedTextField(
                    value = name, onValueChange = { name = it }, singleLine = true,
                    label = { Text(stringResource(R.string.space_name)) },
                )
                Text(stringResource(R.string.space_color), color = c.textSecondary, fontSize = 13.sp)
                Row(horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s8)) {
                    Space.COLORS.forEach { color ->
                        val tint = accentFor(color, dark)
                        Box(
                            Modifier.size(ScydeSize.hitAndroid - ScydeSpace.s8).background(tint.fill, CircleShape)
                                .border(ScydeSpace.s2, if (color == space.color) tint.ring else Color.Transparent, CircleShape)
                                .clickable { browser.recolorSpace(space.id, color) }
                                .semantics { contentDescription = color },
                        )
                    }
                }
                if (browser.spaces.size > 1) {
                    Text(
                        stringResource(R.string.space_delete), color = c.statusDanger, fontSize = 15.sp,
                        modifier = Modifier.clickable { browser.deleteSpace(space.id); onDone() }.padding(vertical = ScydeSpace.s8),
                    )
                }
            }
        },
        confirmButton = { androidx.compose.material3.TextButton(onClick = { browser.renameSpace(space.id, name); onDone() }) { Text(stringResource(R.string.space_rename)) } },
        dismissButton = { androidx.compose.material3.TextButton(onClick = onDone) { Text(stringResource(R.string.omnibox_cancel)) } },
        containerColor = c.surfaceRaised,
    )
}

@Composable
private fun Segment(label: String, selected: Boolean, c: ScydeColors, onClick: () -> Unit) {
    Text(
        label, color = if (selected) c.textPrimary else c.textSecondary, fontSize = 14.sp,
        fontWeight = if (selected) FontWeight.Medium else FontWeight.Normal,
        modifier = Modifier.background(if (selected) c.surfaceRaised else Color.Transparent, CircleShape)
            .clickable(onClick = onClick).padding(horizontal = ScydeSpace.s16, vertical = ScydeSpace.s8),
    )
}
