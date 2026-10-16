package fail.acheron.floatview.ui

import android.content.Intent
import android.os.SystemClock
import android.view.KeyEvent
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.test.platform.app.InstrumentationRegistry
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

class BackNavigationInstrumentedTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    @Test fun systemBackReturnsFromSettingsWithoutLeavingTheActivity() {
        val context = instrumentation.targetContext
        val activity = instrumentation.startActivitySync(
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        ) as MainActivity
        try {
            awaitScript(activity, "!!document.querySelector('nav[aria-label=Primary]') && document.getElementById('app-loading')?.hidden === true")
            script(activity, "Array.from(document.querySelectorAll('[role=alertdialog] button')).find(b => b.textContent.trim() === 'I agree')?.click()")
            awaitScript(activity, "!document.querySelector('[aria-labelledby=safety-title]') && !document.querySelector('main').inert")
            script(activity, "location.hash = '#/settings'")
            awaitScript(activity, "location.hash === '#/settings' && document.querySelector('h1')?.textContent === 'Settings'")
            instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
            awaitScript(activity, "location.hash === '#/' && document.querySelector('h1')?.textContent === 'Connect'")
            script(activity, "location.hash = '#/sessions'")
            awaitScript(activity, "document.querySelector('button[aria-label=\"Session actions\"]') !== null")
            script(activity, "document.querySelector('button[aria-label=\"Session actions\"]').click()")
            script(activity, "Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === 'Prune all').click()")
            awaitScript(activity, "document.querySelector('[role=alertdialog]') !== null")
            instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
            awaitScript(activity, "document.querySelector('[role=alertdialog]') === null && location.hash === '#/sessions'")
            instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
            awaitScript(activity, "location.hash === '#/'")
            instrumentation.runOnMainSync {
                assertFalse(activity.isFinishing)
                assertTrue(activity.frontendIsAttached())
                assertTrue(activity.window.decorView.hasWindowFocus())
            }
        } finally {
            instrumentation.runOnMainSync { activity.finish() }
        }
    }

    private fun script(activity: MainActivity, source: String): String? {
        val done = CountDownLatch(1)
        var result: String? = null
        instrumentation.runOnMainSync {
            val canvas = activity.findViewById<FrameLayout>(android.R.id.content).getChildAt(0) as FrameLayout
            (canvas.getChildAt(1) as WebView).evaluateJavascript(source) {
                result = it
                done.countDown()
            }
        }
        assertTrue("WebView did not answer", done.await(5, TimeUnit.SECONDS))
        return result
    }

    private fun awaitScript(activity: MainActivity, condition: String) {
        val deadline = SystemClock.uptimeMillis() + 15_000
        while (SystemClock.uptimeMillis() < deadline) {
            if (script(activity, condition) == "true") return
            SystemClock.sleep(50)
        }
        fail("Frontend condition did not become true: $condition")
    }
}
