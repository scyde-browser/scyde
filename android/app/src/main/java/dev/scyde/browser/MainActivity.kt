package dev.scyde.browser

import android.content.Intent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.platform.LocalDensity
import kotlinx.coroutines.launch
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import dev.scyde.browser.theme.ScydeAccent
import dev.scyde.browser.theme.ScydeAccents
import dev.scyde.browser.theme.ScydeColors
import dev.scyde.browser.theme.ScydeColorsDark
import dev.scyde.browser.theme.ScydeColorsLight
import dev.scyde.browser.theme.ScydeRadius
import dev.scyde.browser.theme.ScydeSize
import dev.scyde.browser.theme.ScydeSpace
import org.mozilla.geckoview.GeckoView
import uniffi.scyde_core.omniboxDisplayUrl

class MainActivity : ComponentActivity() {
    private lateinit var browser: Browser

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge() // mandatory for targetSdk 35+ (CLAUDE.md)
        super.onCreate(savedInstanceState)
        browser = Browser(this)
        val start = intent?.dataString ?: intent?.getStringExtra("url")
        if (browser.tabs.isEmpty()) browser.newTab(start)
        intent?.getStringExtra("page")?.let { browser.openPage(it) } // CI screenshots
        setContent { ScydeApp(browser) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        intent.dataString?.let { browser.newTab(it) }
        intent.getStringExtra("page")?.let { browser.openPage(it) }
    }
}

private fun accentFor(name: String?, dark: Boolean): ScydeAccent = when (name) {
    "sky" -> if (dark) ScydeAccents.skyDark else ScydeAccents.skyLight
    "periwinkle" -> if (dark) ScydeAccents.periwinkleDark else ScydeAccents.periwinkleLight
    "lavender" -> if (dark) ScydeAccents.lavenderDark else ScydeAccents.lavenderLight
    "mint" -> if (dark) ScydeAccents.mintDark else ScydeAccents.mintLight
    "onyx" -> if (dark) ScydeAccents.onyxDark else ScydeAccents.onyxLight
    else -> if (dark) ScydeAccents.emberDark else ScydeAccents.emberLight
}

@Composable
fun ScydeApp(browser: Browser) {
    val system = isSystemInDarkTheme()
    val dark = when (browser.appearance["theme"]) { "dark" -> true; "light" -> false; else -> system }
    val c = if (dark) ScydeColorsDark else ScydeColorsLight
    val accent = accentFor(browser.appearance["accent"], dark)
    var showTabs by remember { mutableStateOf(false) }
    val active = browser.active
    val sheet = browser.sheetPage
    var edgeSide by remember { mutableIntStateOf(0) }
    var edgeFraction by remember { mutableFloatStateOf(0f) }
    val haptic = LocalHapticFeedback.current
    val density = LocalDensity.current

    // Predictive back (CLAUDE.md): sheet → tab grid → page history → exit.
    BackHandler(enabled = sheet != null || showTabs || active?.canGoBack == true) {
        when {
            sheet != null -> browser.closeSheet()
            showTabs -> showTabs = false
            else -> active?.session?.goBack()
        }
    }

    Box(Modifier.fillMaxSize().background(c.surfaceCanvas)) {
        Column(Modifier.fillMaxSize()) {
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
            Island(
                browser, c, accent,
                onTabs = { showTabs = true },
                modifier = Modifier.align(Alignment.BottomCenter).navigationBarsPadding().imePadding()
                    .padding(horizontal = ScydeSize.islandMobileInset, vertical = ScydeSpace.s8),
            )
        }
    }
    if (showTabs) TabGrid(browser, c, accent, onDone = { showTabs = false })
}

/** Title and × for Settings, Library and Shield (they are sheets, not tabs). */
@Composable
private fun SheetBar(page: String, c: ScydeColors, onClose: () -> Unit) {
    val title = when (page) {
        "library" -> stringResource(R.string.library_title)
        "shields" -> stringResource(R.string.shield_label)
        else -> stringResource(R.string.rail_settings)
    }
    Row(
        Modifier.fillMaxWidth().background(c.surfaceRaised).statusBarsPadding().height(ScydeSize.hitAndroid + ScydeSpace.s8)
            .padding(horizontal = ScydeSpace.s8),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, color = c.textPrimary, fontSize = 18.sp, modifier = Modifier.weight(1f).padding(horizontal = ScydeSpace.s12))
        IslandButton(stringResource(R.string.sheet_close), c, onClick = onClose) { Text("×", color = c.textPrimary, fontSize = 26.sp) }
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
    ) { Text(if (side < 0) "‹" else "›", color = c.islandText, fontSize = 26.sp) }
}

@Composable
private fun Island(browser: Browser, c: ScydeColors, accent: ScydeAccent, onTabs: () -> Unit, modifier: Modifier) {
    val tab = browser.active
    var editing by remember { mutableStateOf(false) }
    var menu by remember { mutableStateOf(false) }
    var field by remember { mutableStateOf(TextFieldValue("")) }
    val focus = remember { FocusRequester() }
    val haptic = LocalHapticFeedback.current
    val shape = RoundedCornerShape(ScydeRadius.island)
    val drag = remember { Animatable(0f) }
    val scope = rememberCoroutineScope()

    Row(
        modifier
            .fillMaxWidth()
            .widthIn(max = ScydeSize.islandWidth)
            .height(ScydeSize.islandMobileHeight)
            .shadow(ScydeSpace.s16, shape)
            .background(c.islandBg, shape)
            .border(ScydeSpace.s2 / 2, c.islandBorder, shape)
            .padding(horizontal = ScydeSpace.s8)
            .graphicsLayer { translationX = drag.value }
            // Watches in the Initial pass, before the buttons: a drag that starts on a button (most of
            // the island) still switches tabs, and once it is a drag the buttons do not get a click.
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
                        if (!dragging && !editing && (kotlin.math.abs(dx) > slop || kotlin.math.abs(dy) > slop)) dragging = true
                        if (dragging) {
                            ch.consume()
                            if (kotlin.math.abs(dx) > kotlin.math.abs(dy)) scope.launch { drag.snapTo(dx * 0.35f) }
                        }
                    }
                    scope.launch { drag.animateTo(0f, spring(dampingRatio = 0.7f, stiffness = Spring.StiffnessMedium)) }
                    if (!dragging) return@awaitEachGesture
                    if (dy < -ScydeSpace.s40.toPx() && kotlin.math.abs(dy) > kotlin.math.abs(dx)) onTabs()
                    else if (kotlin.math.abs(dx) > ScydeSpace.s48.toPx()) {
                        // Like Safari: left → next tab (a new one after the last), right → previous.
                        val i = browser.tabs.indexOfFirst { it.id == browser.activeId }
                        val j = i + if (dx < 0) 1 else -1
                        when {
                            i < 0 -> Unit
                            j >= browser.tabs.size -> browser.newTab()
                            j >= 0 -> browser.activeId = browser.tabs[j].id
                        }
                        haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    }
                }
            },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s4),
    ) {
        if (!editing) {
            IslandButton(stringResource(R.string.nav_back), c, enabled = tab?.canGoBack == true, onClick = { tab?.session?.goBack() }) {
                Text("‹", color = if (tab?.canGoBack == true) c.islandText else c.islandTextSecondary, fontSize = 26.sp)
            }
        }
        Box(Modifier.weight(1f).height(ScydeSize.hitAndroid), contentAlignment = Alignment.CenterStart) {
            if (editing) {
                LaunchedEffect(Unit) { focus.requestFocus() }
                BasicTextField(
                    value = field,
                    onValueChange = { v ->
                        val grew = v.text.length > field.text.length && v.selection.collapsed
                        field = v
                        if (grew) browser.suggestion(v.text)?.let { rest ->
                            field = TextFieldValue(v.text + rest, TextRange(v.text.length, v.text.length + rest.length))
                        }
                    },
                    singleLine = true,
                    textStyle = TextStyle(color = c.islandText, fontSize = 16.sp),
                    cursorBrush = SolidColor(accent.fill),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go, autoCorrectEnabled = false),
                    keyboardActions = KeyboardActions(onGo = { browser.go(field.text); editing = false }),
                    modifier = Modifier.fillMaxWidth().focusRequester(focus)
                        .background(c.islandRaised, CircleShape).padding(horizontal = ScydeSpace.s12, vertical = ScydeSpace.s12)
                        .semantics { contentDescription = "address" },
                )
            } else {
                val page = tab?.url?.let { browser.internalPage(it) }
                val display = if (page == null && tab != null && tab.url.startsWith("http")) omniboxDisplayUrl(tab.url) else null
                val label = when {
                    page == "newtab" || tab == null || tab.url.isEmpty() -> null
                    page != null -> "scyde://$page"
                    display != null -> display.domain
                    else -> tab.url
                }
                Text(
                    text = label ?: stringResource(R.string.omnibox_placeholder),
                    color = if (label == null) c.islandTextSecondary else c.islandText,
                    fontSize = 16.sp,
                    maxLines = 1,
                    overflow = TextOverflow.StartEllipsis,
                    modifier = Modifier.fillMaxWidth().clickable {
                        val u = tab?.url ?: ""
                        val text = if (page != null) (if (page == "newtab") "" else "scyde://$page") else u
                        field = TextFieldValue(text, TextRange(0, text.length))
                        editing = true
                    }.padding(horizontal = ScydeSpace.s12),
                )
            }
            val p = tab?.progress ?: 100
            if (p in 1..99) {
                Box(Modifier.align(Alignment.BottomStart).fillMaxWidth(p / 100f).height(ScydeSpace.s2).background(accent.fill, CircleShape))
            }
        }
        if (!editing) {
            IslandButton(stringResource(R.string.shield_label), c, onClick = { browser.openPage("shields") }) {
                Box(Modifier.size(ScydeSpace.s8).background(if (browser.shieldReady) c.islandShield else c.islandTextSecondary, CircleShape))
            }
            IslandButton(stringResource(R.string.mobile_tabs), c, onClick = onTabs) {
                Box(
                    Modifier.size(ScydeSpace.s24).border(ScydeSpace.s2 * 0.75f, c.islandText, RoundedCornerShape(ScydeRadius.xs)),
                    contentAlignment = Alignment.Center,
                ) { Text("${browser.tabs.size}", color = c.islandText, fontSize = 12.sp) }
            }
            Box {
                IslandButton(stringResource(R.string.mobile_menu), c, onClick = { menu = true }) { Text("⋯", color = c.islandText, fontSize = 22.sp) }
                DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.island_new_tab)) }, onClick = { menu = false; browser.newTab() })
                    DropdownMenuItem(text = { Text(stringResource(R.string.library_tab_history)) }, onClick = { menu = false; browser.openPage("library") })
                    DropdownMenuItem(text = { Text(stringResource(R.string.shield_label)) }, onClick = { menu = false; browser.openPage("shields") })
                    DropdownMenuItem(text = { Text(stringResource(R.string.rail_settings)) }, onClick = { menu = false; browser.openPage("settings") })
                }
            }
        }
    }
    BackHandler(enabled = editing) { editing = false }
}

@Composable
private fun IslandButton(label: String, c: ScydeColors, enabled: Boolean = true, onClick: () -> Unit, content: @Composable () -> Unit) {
    Box(
        Modifier.size(ScydeSize.hitAndroid).clickable(enabled = enabled, onClick = onClick).semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) { content() }
}

@Composable
private fun TabGrid(browser: Browser, c: ScydeColors, accent: ScydeAccent, onDone: () -> Unit) {
    Dialog(onDismissRequest = onDone, properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Column(Modifier.fillMaxSize().background(c.surfaceCanvas).windowInsetsPadding(WindowInsets.safeDrawing)) {
            Row(Modifier.fillMaxWidth().padding(ScydeSpace.s16), verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.mobile_tabs_close_all), color = c.statusDanger, modifier = Modifier.clickable { browser.closeAll(); onDone() }.padding(ScydeSpace.s12))
                Box(Modifier.weight(1f), contentAlignment = Alignment.Center) {
                    Text("+", color = c.textPrimary, fontSize = 24.sp, modifier = Modifier.clickable { browser.newTab(); onDone() }.padding(ScydeSpace.s12).semantics { contentDescription = "new tab" })
                }
                Text(stringResource(R.string.mobile_tabs_done), color = accent.ink, modifier = Modifier.clickable(onClick = onDone).padding(ScydeSpace.s12))
            }
            LazyVerticalGrid(columns = GridCells.Adaptive(ScydeSpace.s80 * 2), contentPadding = androidx.compose.foundation.layout.PaddingValues(ScydeSpace.s16),
                horizontalArrangement = Arrangement.spacedBy(ScydeSpace.s12), verticalArrangement = Arrangement.spacedBy(ScydeSpace.s12)) {
                items(browser.tabs, key = { it.id }) { tab ->
                    val on = tab.id == browser.activeId
                    Column(
                        Modifier.background(c.surfaceRaised, RoundedCornerShape(ScydeRadius.md))
                            .border(ScydeSpace.s2, if (on) accent.ring else c.surfaceRaised, RoundedCornerShape(ScydeRadius.md))
                            .clickable { browser.activeId = tab.id; onDone() }.padding(ScydeSpace.s8),
                    ) {
                        Box(Modifier.fillMaxWidth().aspectRatio(0.75f).background(c.surfaceSunken, RoundedCornerShape(ScydeRadius.sm)), contentAlignment = Alignment.Center) {
                            Text(tab.url.substringAfter("://").substringBefore('/'), color = c.textTertiary, fontSize = 11.sp, maxLines = 1)
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(tab.title.ifEmpty { stringResource(R.string.tab_new) }, color = c.textPrimary, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f).padding(top = ScydeSpace.s8))
                            Text("×", color = c.textSecondary, fontSize = 20.sp, modifier = Modifier.clickable { browser.close(tab) }.padding(ScydeSpace.s8).semantics { contentDescription = "close tab" })
                        }
                    }
                }
            }
        }
    }
}
