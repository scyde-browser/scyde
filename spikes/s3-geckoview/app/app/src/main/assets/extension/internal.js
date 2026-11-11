/* global browser */
// Internal page (scyde://newtab is served from the built-in extension on Android) -> app via native messaging.
(async () => {
  const status = document.getElementById('status');
  try {
    const reply = await browser.runtime.sendNativeMessage('scyde', { type: 'internal-ping', page: location.href });
    status.textContent = `Aplikacija je odgovorila: ${JSON.stringify(reply)}`;
    await browser.runtime.sendNativeMessage('scyde', { type: 'internal-hello', reply });
  } catch (e) {
    status.textContent = `Greška: ${e}`;
    browser.runtime.sendNativeMessage('scyde', { type: 'error', error: String(e) });
  }
})();
