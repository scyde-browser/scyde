package dev.scyde.spike.gecko

import android.content.Intent
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** G3: extension blocks the fixture, WASM p99 < 200 µs in SpiderMonkey, internal page talks to the app. */
@RunWith(AndroidJUnit4::class)
class G3Test {
    @Test
    fun shieldExtensionWasmAndNativeMessaging() {
        val ctx = InstrumentationRegistry.getInstrumentation().targetContext
        val intent = Intent(ctx, MainActivity::class.java)
            .putExtra("url", "http://10.0.2.2:8787/adblock/")
            .putExtra("internal", true)
        ActivityScenario.launch<MainActivity>(intent).use {
            assertNotNull("extension did not install", ResultBus.await("extension", 60))

            val fixture = ResultBus.await("fixture", 90)
            assertNotNull("fixture page did not report", fixture)
            val results = fixture!!.getJSONObject("results")
            for (key in results.keys()) {
                if (key.startsWith("net:")) assertTrue("network rule not applied: $key", results.getBoolean(key))
            }
            assertTrue(results.getBoolean("allow:exception"))
            assertTrue(results.getBoolean("keep:content"))

            val bench = ResultBus.await("bench", 120)
            assertNotNull("no WASM benchmark from background", bench)
            assertTrue("G3: WASM p99 must be < 200 µs, was ${bench!!.getDouble("p99_us")}", bench.getDouble("p99_us") < 200.0)

            val internal = ResultBus.await("internal-hello", 60)
            assertNotNull("internal page did not reach the app via native messaging", internal)
            assertEquals("android", internal!!.optJSONObject("reply")?.optString("from"))
        }
    }
}
