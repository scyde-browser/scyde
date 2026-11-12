package dev.scyde.browser

import android.app.DatePickerDialog
import android.app.TimePickerDialog
import android.net.Uri
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import dev.scyde.browser.theme.ScydeType
import dev.scyde.browser.theme.ScydeColors
import dev.scyde.browser.theme.ScydeSize
import dev.scyde.browser.theme.ScydeSpace
import org.mozilla.geckoview.AllowOrDeny
import org.mozilla.geckoview.GeckoResult
import org.mozilla.geckoview.GeckoSession
import org.mozilla.geckoview.GeckoSession.PermissionDelegate
import org.mozilla.geckoview.GeckoSession.PromptDelegate
import java.util.Calendar

/**
 * What a page asks the user (GeckoView shows nothing by itself): alerts, confirms, text prompts,
 * `<select>` lists, HTTP sign-in, date and time pickers, and permission questions. One at a time;
 * [done] must be called exactly once, with `null` when the user backs out.
 */
sealed class PagePrompt {
    /** Answers the page as if the user backed out (a newer question replaced this one). */
    fun cancel() = when (this) {
        is Alert -> done()
        is Confirm -> done(false)
        is Text -> done(null)
        is Choice -> done(null)
        is Auth -> done(null)
        is DateTime -> done(null)
    }

    class Alert(val title: String?, val message: String?, val done: () -> Unit) : PagePrompt()
    class Confirm(val title: String?, val message: String?, val yes: String?, val no: String?, val done: (Boolean) -> Unit) : PagePrompt()
    class Text(val title: String?, val message: String?, val value: String, val done: (String?) -> Unit) : PagePrompt()
    class Choice(val title: String?, val items: List<Item>, val multiple: Boolean, val done: (List<String>?) -> Unit) : PagePrompt() {
        class Item(val id: String, val label: String, val selected: Boolean, val disabled: Boolean, val header: Boolean)
    }
    class Auth(val title: String?, val message: String?, val onlyPassword: Boolean, val username: String, val done: (Pair<String, String>?) -> Unit) : PagePrompt()
    class DateTime(val time: Boolean, val value: String, val done: (String?) -> Unit) : PagePrompt()
}

/** GeckoView's prompt and permission requests, turned into [PagePrompt]s on [Browser.prompt]. */
class PagePrompts(private val browser: Browser, private val countPopup: () -> Unit) : PromptDelegate, PermissionDelegate {
    private fun <T : Any> ask(build: (GeckoResult<T>) -> PagePrompt): GeckoResult<T> {
        val result = GeckoResult<T>()
        // A second question while one is open: Browser.prompt answers the first as cancelled.
        browser.prompt = build(result)
        return result
    }

    override fun onPopupPrompt(session: GeckoSession, prompt: PromptDelegate.PopupPrompt): GeckoResult<PromptDelegate.PromptResponse> {
        // Gecko blocks window.open without a gesture and asks here: always deny and count (PLAN §5.5).
        countPopup()
        return GeckoResult.fromValue(prompt.confirm(AllowOrDeny.DENY))
    }

    override fun onAlertPrompt(session: GeckoSession, prompt: PromptDelegate.AlertPrompt): GeckoResult<PromptDelegate.PromptResponse> =
        ask { r -> PagePrompt.Alert(prompt.title, prompt.message) { r.complete(prompt.dismiss()) } }

    override fun onButtonPrompt(session: GeckoSession, prompt: PromptDelegate.ButtonPrompt): GeckoResult<PromptDelegate.PromptResponse> =
        ask { r ->
            PagePrompt.Confirm(prompt.title, prompt.message, null, null) { yes ->
                r.complete(prompt.confirm(if (yes) PromptDelegate.ButtonPrompt.Type.POSITIVE else PromptDelegate.ButtonPrompt.Type.NEGATIVE))
            }
        }

    override fun onTextPrompt(session: GeckoSession, prompt: PromptDelegate.TextPrompt): GeckoResult<PromptDelegate.PromptResponse> =
        ask { r ->
            PagePrompt.Text(prompt.title, prompt.message, prompt.defaultValue ?: "") { text ->
                r.complete(if (text != null) prompt.confirm(text) else prompt.dismiss())
            }
        }

    override fun onChoicePrompt(session: GeckoSession, prompt: PromptDelegate.ChoicePrompt): GeckoResult<PromptDelegate.PromptResponse> {
        val items = mutableListOf<PagePrompt.Choice.Item>()
        fun add(choices: Array<PromptDelegate.ChoicePrompt.Choice>) {
            for (c in choices) {
                if (c.separator) continue
                val group = c.items
                items += PagePrompt.Choice.Item(c.id, c.label, c.selected, c.disabled, header = group != null)
                if (group != null) add(group)
            }
        }
        add(prompt.choices)
        val multiple = prompt.type == PromptDelegate.ChoicePrompt.Type.MULTIPLE
        return ask { r ->
            PagePrompt.Choice(prompt.title, items, multiple) { ids ->
                r.complete(
                    when {
                        ids == null -> prompt.dismiss()
                        multiple -> prompt.confirm(ids.toTypedArray())
                        ids.isEmpty() -> prompt.dismiss()
                        else -> prompt.confirm(ids.first())
                    },
                )
            }
        }
    }

    override fun onAuthPrompt(session: GeckoSession, prompt: PromptDelegate.AuthPrompt): GeckoResult<PromptDelegate.PromptResponse> {
        val options = prompt.authOptions
        val onlyPassword = options.flags and PromptDelegate.AuthPrompt.AuthOptions.Flags.ONLY_PASSWORD != 0
        return ask { r ->
            PagePrompt.Auth(prompt.title, prompt.message, onlyPassword, options.username ?: "") { creds ->
                r.complete(
                    when {
                        creds == null -> prompt.dismiss()
                        onlyPassword -> prompt.confirm(creds.second)
                        else -> prompt.confirm(creds.first, creds.second)
                    },
                )
            }
        }
    }

    override fun onDateTimePrompt(session: GeckoSession, prompt: PromptDelegate.DateTimePrompt): GeckoResult<PromptDelegate.PromptResponse> {
        val time = prompt.type == PromptDelegate.DateTimePrompt.Type.TIME
        // Date, month and week all start from a date; date-time fields get the date part.
        return ask { r ->
            PagePrompt.DateTime(time, prompt.defaultValue ?: "") { value ->
                val formatted = when {
                    value == null -> null
                    prompt.type == PromptDelegate.DateTimePrompt.Type.MONTH -> value.take(7)
                    prompt.type == PromptDelegate.DateTimePrompt.Type.DATETIME_LOCAL -> "${value}T00:00"
                    prompt.type == PromptDelegate.DateTimePrompt.Type.WEEK -> null // no native week picker
                    else -> value
                }
                r.complete(if (formatted != null) prompt.confirm(formatted) else prompt.dismiss())
            }
        }
    }

    override fun onFilePrompt(session: GeckoSession, prompt: PromptDelegate.FilePrompt): GeckoResult<PromptDelegate.PromptResponse>? {
        val pick = browser.pickFiles ?: return null
        val result = GeckoResult<PromptDelegate.PromptResponse>()
        val multiple = prompt.type == PromptDelegate.FilePrompt.Type.MULTIPLE
        val types = prompt.mimeTypes?.filter { it.contains('/') }?.toTypedArray()?.takeIf { it.isNotEmpty() } ?: arrayOf("*/*")
        pick(types, multiple) { uris ->
            result.complete(
                when {
                    uris.isEmpty() -> prompt.dismiss()
                    multiple -> prompt.confirm(browser.appContext, uris.toTypedArray())
                    else -> prompt.confirm(browser.appContext, uris.first())
                },
            )
        }
        return result
    }

    override fun onBeforeUnloadPrompt(session: GeckoSession, prompt: PromptDelegate.BeforeUnloadPrompt): GeckoResult<PromptDelegate.PromptResponse> =
        ask { r ->
            PagePrompt.Confirm(browser.string(R.string.prompt_leave_title), browser.string(R.string.prompt_leave_body), browser.string(R.string.prompt_leave_confirm), browser.string(R.string.prompt_leave_stay)) { leave ->
                r.complete(prompt.confirm(if (leave) AllowOrDeny.ALLOW else AllowOrDeny.DENY))
            }
        }

    override fun onRepostConfirmPrompt(session: GeckoSession, prompt: PromptDelegate.RepostConfirmPrompt): GeckoResult<PromptDelegate.PromptResponse> =
        ask { r ->
            PagePrompt.Confirm(browser.string(R.string.prompt_repost_title), browser.string(R.string.prompt_repost_body), browser.string(R.string.prompt_repost_confirm), null) { again ->
                r.complete(prompt.confirm(if (again) AllowOrDeny.ALLOW else AllowOrDeny.DENY))
            }
        }

    // ------------------------------------------------------------ permissions

    private fun siteOf(uri: String): String = runCatching { Uri.parse(uri).host }.getOrNull() ?: uri

    /** Android runtime permissions (after the user allowed the site): asked through the Activity. */
    override fun onAndroidPermissionsRequest(session: GeckoSession, permissions: Array<String>?, callback: PermissionDelegate.Callback) {
        val request = browser.requestAndroidPermissions
        if (permissions.isNullOrEmpty() || request == null) return callback.reject()
        request(permissions) { granted -> if (granted) callback.grant() else callback.reject() }
    }

    override fun onContentPermissionRequest(session: GeckoSession, perm: PermissionDelegate.ContentPermission): GeckoResult<Int>? {
        when (perm.permission) {
            PermissionDelegate.PERMISSION_GEOLOCATION -> Unit
            // Muted autoplay (video previews) plays; sound waits for a tap.
            PermissionDelegate.PERMISSION_AUTOPLAY_INAUDIBLE, PermissionDelegate.PERMISSION_MEDIA_KEY_SYSTEM_ACCESS ->
                return GeckoResult.fromValue(PermissionDelegate.ContentPermission.VALUE_ALLOW)
            // Notifications, cross-site storage access, persistent storage, audible autoplay: no.
            else -> return GeckoResult.fromValue(PermissionDelegate.ContentPermission.VALUE_DENY)
        }
        return ask { r ->
            PagePrompt.Confirm(null, browser.string(R.string.permission_location, siteOf(perm.uri)), browser.string(R.string.prompt_allow), browser.string(R.string.prompt_deny)) { allow ->
                r.complete(if (allow) PermissionDelegate.ContentPermission.VALUE_ALLOW else PermissionDelegate.ContentPermission.VALUE_DENY)
            }
        }
    }

    override fun onMediaPermissionRequest(
        session: GeckoSession,
        uri: String,
        video: Array<PermissionDelegate.MediaSource>?,
        audio: Array<PermissionDelegate.MediaSource>?,
        callback: PermissionDelegate.MediaCallback,
    ) {
        val message = when {
            !video.isNullOrEmpty() && !audio.isNullOrEmpty() -> R.string.permission_camera_microphone
            !video.isNullOrEmpty() -> R.string.permission_camera
            else -> R.string.permission_microphone
        }
        browser.prompt = PagePrompt.Confirm(null, browser.string(message, siteOf(uri)), browser.string(R.string.prompt_allow), browser.string(R.string.prompt_deny)) { allow ->
            if (allow) callback.grant(video?.firstOrNull(), audio?.firstOrNull()) else callback.reject()
        }
    }
}

private fun titleOf(title: String?): (@Composable () -> Unit)? {
    val text = title?.takeIf { it.isNotBlank() } ?: return null
    return { Text(text) }
}

/** Shows [Browser.prompt] as a dialog. Dismissing it answers the page with "cancelled". */
@Composable
fun PromptHost(browser: Browser, c: ScydeColors) {
    val prompt = browser.prompt ?: return
    val ok = stringResource(R.string.prompt_ok)
    val cancel = stringResource(R.string.omnibox_cancel)
    fun close() {
        if (browser.prompt === prompt) browser.prompt = null
    }
    when (prompt) {
        is PagePrompt.Alert -> AlertDialog(
            onDismissRequest = { prompt.done(); close() },
            title = titleOf(prompt.title),
            text = { Text(prompt.message ?: "") },
            confirmButton = { TextButton(onClick = { prompt.done(); close() }) { Text(ok) } },
            containerColor = c.surfaceRaised,
        )
        is PagePrompt.Confirm -> AlertDialog(
            onDismissRequest = { prompt.done(false); close() },
            title = titleOf(prompt.title),
            text = { Text(prompt.message ?: "") },
            confirmButton = { TextButton(onClick = { prompt.done(true); close() }) { Text(prompt.yes ?: ok) } },
            dismissButton = { TextButton(onClick = { prompt.done(false); close() }) { Text(prompt.no ?: cancel) } },
            containerColor = c.surfaceRaised,
        )
        is PagePrompt.Text -> {
            var text by remember(prompt) { mutableStateOf(prompt.value) }
            AlertDialog(
                onDismissRequest = { prompt.done(null); close() },
                title = titleOf(prompt.title),
                text = {
                    Column(verticalArrangement = Arrangement.spacedBy(ScydeSpace.s12)) {
                        if (!prompt.message.isNullOrBlank()) Text(prompt.message)
                        OutlinedTextField(value = text, onValueChange = { text = it }, singleLine = true)
                    }
                },
                confirmButton = { TextButton(onClick = { prompt.done(text); close() }) { Text(ok) } },
                dismissButton = { TextButton(onClick = { prompt.done(null); close() }) { Text(cancel) } },
                containerColor = c.surfaceRaised,
            )
        }
        is PagePrompt.Choice -> {
            var picked by remember(prompt) { mutableStateOf(prompt.items.filter { it.selected }.map { it.id }.toSet()) }
            val maxHeight = (LocalConfiguration.current.screenHeightDp * 0.6f).dp
            AlertDialog(
                onDismissRequest = { prompt.done(null); close() },
                title = { Text(prompt.title?.takeIf { it.isNotBlank() } ?: stringResource(R.string.prompt_choose)) },
                text = {
                    Column(Modifier.heightIn(max = maxHeight).verticalScroll(rememberScrollState())) {
                        prompt.items.forEach { item ->
                            if (item.header) {
                                Text(item.label, color = c.textTertiary, fontSize = ScydeType.captionSize, modifier = Modifier.padding(vertical = ScydeSpace.s8))
                            } else {
                                Row(
                                    Modifier.fillMaxWidth().heightIn(min = ScydeSize.hitAndroid)
                                        .clickable(enabled = !item.disabled) {
                                            if (prompt.multiple) {
                                                picked = if (item.id in picked) picked - item.id else picked + item.id
                                            } else {
                                                prompt.done(listOf(item.id))
                                                close()
                                            }
                                        },
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    if (prompt.multiple) Checkbox(checked = item.id in picked, onCheckedChange = null) else RadioButton(selected = item.id in picked, onClick = null)
                                    Text(item.label, color = if (item.disabled) c.textTertiary else c.textPrimary, modifier = Modifier.padding(start = ScydeSpace.s12))
                                }
                            }
                        }
                    }
                },
                confirmButton = {
                    if (prompt.multiple) TextButton(onClick = { prompt.done(picked.toList()); close() }) { Text(ok) }
                },
                dismissButton = { TextButton(onClick = { prompt.done(null); close() }) { Text(cancel) } },
                containerColor = c.surfaceRaised,
            )
        }
        is PagePrompt.Auth -> {
            var user by remember(prompt) { mutableStateOf(prompt.username) }
            var pass by remember(prompt) { mutableStateOf("") }
            AlertDialog(
                onDismissRequest = { prompt.done(null); close() },
                title = { Text(prompt.title?.takeIf { it.isNotBlank() } ?: stringResource(R.string.auth_title)) },
                text = {
                    Column(verticalArrangement = Arrangement.spacedBy(ScydeSpace.s12)) {
                        if (!prompt.message.isNullOrBlank()) Text(prompt.message)
                        if (!prompt.onlyPassword) {
                            OutlinedTextField(value = user, onValueChange = { user = it }, singleLine = true, label = { Text(stringResource(R.string.auth_username)) })
                        }
                        OutlinedTextField(
                            value = pass, onValueChange = { pass = it }, singleLine = true,
                            label = { Text(stringResource(R.string.auth_password)) },
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                        )
                    }
                },
                confirmButton = { TextButton(onClick = { prompt.done(user to pass); close() }) { Text(stringResource(R.string.auth_confirm)) } },
                dismissButton = { TextButton(onClick = { prompt.done(null); close() }) { Text(cancel) } },
                containerColor = c.surfaceRaised,
            )
        }
        is PagePrompt.DateTime -> {
            // The platform pickers (they follow the system theme and locale).
            val context = LocalContext.current
            LaunchedEffect(prompt) {
                val now = Calendar.getInstance()
                var answered = false
                fun answer(value: String?) {
                    if (answered) return
                    answered = true
                    prompt.done(value)
                    close()
                }
                if (prompt.time) {
                    val parts = prompt.value.split(':').mapNotNull { it.toIntOrNull() }
                    TimePickerDialog(
                        context,
                        { _, h, m -> answer("%02d:%02d".format(h, m)) },
                        parts.getOrElse(0) { now.get(Calendar.HOUR_OF_DAY) }, parts.getOrElse(1) { now.get(Calendar.MINUTE) }, true,
                    ).apply { setOnDismissListener { answer(null) } }.show()
                } else {
                    val parts = prompt.value.take(10).split('-').mapNotNull { it.toIntOrNull() }
                    DatePickerDialog(
                        context,
                        { _, y, m, d -> answer("%04d-%02d-%02d".format(y, m + 1, d)) },
                        parts.getOrElse(0) { now.get(Calendar.YEAR) }, parts.getOrElse(1) { now.get(Calendar.MONTH) + 1 } - 1, parts.getOrElse(2) { now.get(Calendar.DAY_OF_MONTH) },
                    ).apply { setOnDismissListener { answer(null) } }.show()
                }
            }
        }
    }
}
