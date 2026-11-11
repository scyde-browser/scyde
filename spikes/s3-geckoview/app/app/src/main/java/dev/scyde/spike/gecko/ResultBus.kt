package dev.scyde.spike.gecko

import android.util.Log
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Collects what the built-in extension reports, so the instrumented test can assert on it. */
object ResultBus {
    private val values = ConcurrentHashMap<String, JSONObject>()
    private val latches = ConcurrentHashMap<String, CountDownLatch>()

    private fun latch(key: String) = latches.getOrPut(key) { CountDownLatch(1) }

    fun put(key: String, value: JSONObject) {
        values[key] = value
        Log.i("SCYDE", "SCYDE_RESULT $key $value")
        latch(key).countDown()
    }

    fun await(key: String, seconds: Long): JSONObject? =
        if (latch(key).await(seconds, TimeUnit.SECONDS)) values[key] else null
}
