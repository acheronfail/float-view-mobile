package fail.acheron.floatview.ui

import android.app.Activity
import android.app.Application
import android.content.ComponentCallbacks2
import android.content.Intent
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.View
import android.widget.FrameLayout
import androidx.test.platform.app.InstrumentationRegistry
import fail.acheron.floatview.application.ThemePreference
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.theme.AndroidThemeController
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

class StartupPresentationInstrumentedTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    @Test fun nativeCanvasFollowsThemeAndActivityRecreationKeepsRuntime() {
        val app = instrumentation.targetContext.applicationContext as FloatViewApplication
        val runtime = app.runtime
        var activity = instrumentation.startActivitySync(
            Intent(app, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        ) as MainActivity
        try {
            awaitFrontendPaint(activity)
            instrumentation.runOnMainSync {
                val content = activity.findViewById<FrameLayout>(android.R.id.content)
                val canvas = content.getChildAt(0) as FrameLayout
                val theme = AndroidThemeController(activity, canvas)
                for ((preference, color) in listOf(
                    ThemePreference.DARK to Color.rgb(2, 6, 23),
                    ThemePreference.LIGHT to Color.rgb(248, 250, 252),
                )) {
                    theme.apply(preference)
                    assertEquals(color, (canvas.background as ColorDrawable).color)
                    assertEquals(color, (activity.window.decorView.background as ColorDrawable).color)
                    assertTrue(activity.frontendIsAttached())
                }
                theme.apply(runtime.service.theme)
            }
            // Drive lifecycle callbacks deterministically so OS memory policy cannot
            // turn the retained-resume case into an immediate trim-memory case.
            instrumentation.runOnMainSync {
                val canvas = activity.findViewById<FrameLayout>(android.R.id.content).getChildAt(0) as FrameLayout
                val retained = canvas.getChildAt(1)
                instrumentation.callActivityOnPause(activity)
                instrumentation.callActivityOnStop(activity)
                assertTrue(activity.frontendIsAttached())
                instrumentation.callActivityOnStart(activity)
                instrumentation.callActivityOnResume(activity)
                assertSame(retained, canvas.getChildAt(1))
            }
            val graceExpired = CountDownLatch(1)
            instrumentation.runOnMainSync {
                instrumentation.callActivityOnPause(activity)
                instrumentation.callActivityOnStop(activity)
                Handler(Looper.getMainLooper()).postDelayed(
                    { graceExpired.countDown() }, MainActivity.FRONTEND_BACKGROUND_GRACE_MS + 250,
                )
            }
            assertTrue("background grace interval did not expire", graceExpired.await(35, TimeUnit.SECONDS))
            instrumentation.runOnMainSync {
                val canvas = activity.findViewById<FrameLayout>(android.R.id.content).getChildAt(0) as FrameLayout
                assertFalse(activity.frontendIsAttached())
                assertEquals(1, canvas.childCount)
                assertTrue(canvas.getChildAt(0).isShown)
                instrumentation.callActivityOnStart(activity)
                instrumentation.callActivityOnResume(activity)
                assertTrue(activity.frontendIsAttached())
                assertEquals(2, canvas.childCount)
                activity.onTrimMemory(ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL)
                assertFalse(activity.frontendIsAttached())
                assertEquals(1, canvas.childCount)
                assertTrue(canvas.getChildAt(0).isShown)
                instrumentation.callActivityOnPause(activity)
                instrumentation.callActivityOnStop(activity)
                instrumentation.callActivityOnStart(activity)
                instrumentation.callActivityOnResume(activity)
                assertTrue(activity.frontendIsAttached())
                assertEquals(2, canvas.childCount)
                assertSame(runtime, app.runtime)
            }
            repeat(3) {
                val previous = activity
                val resumed = CountDownLatch(1)
                var replacement: MainActivity? = null
                val callback = object : Application.ActivityLifecycleCallbacks {
                    override fun onActivityResumed(value: Activity) {
                        if (value is MainActivity && value !== previous) {
                            replacement = value
                            resumed.countDown()
                        }
                    }
                    override fun onActivityCreated(value: Activity, state: Bundle?) {}
                    override fun onActivityStarted(value: Activity) {}
                    override fun onActivityPaused(value: Activity) {}
                    override fun onActivityStopped(value: Activity) {}
                    override fun onActivitySaveInstanceState(value: Activity, state: Bundle) {}
                    override fun onActivityDestroyed(value: Activity) {}
                }
                app.registerActivityLifecycleCallbacks(callback)
                try {
                    instrumentation.runOnMainSync { previous.recreate() }
                    assertTrue("replacement Activity did not resume", resumed.await(10, TimeUnit.SECONDS))
                    instrumentation.waitForIdleSync()
                    activity = requireNotNull(replacement)
                    instrumentation.runOnMainSync {
                        assertSame(runtime, app.runtime)
                        assertFalse(previous.frontendIsAttached())
                        assertTrue(activity.frontendIsAttached())
                        val canvas = activity.findViewById<FrameLayout>(android.R.id.content).getChildAt(0) as FrameLayout
                        assertEquals("recreation must attach one placeholder and one WebView", 2, canvas.childCount)
                    }
                    awaitFrontendPaint(activity)
                } finally {
                    app.unregisterActivityLifecycleCallbacks(callback)
                }
            }
        } finally {
            instrumentation.runOnMainSync { activity.finish() }
        }
    }

    private fun awaitFrontendPaint(activity: MainActivity) {
        val painted = CountDownLatch(1)
        val deadline = SystemClock.uptimeMillis() + 10_000
        instrumentation.runOnMainSync {
            val handler = Handler(Looper.getMainLooper())
            val poll = object : Runnable {
                override fun run() {
                    val canvas = activity.findViewById<FrameLayout>(android.R.id.content).getChildAt(0) as FrameLayout
                    if (canvas.childCount == 2 && canvas.getChildAt(0).visibility == View.GONE &&
                        canvas.getChildAt(1).visibility == View.VISIBLE
                    ) {
                        painted.countDown()
                    } else if (SystemClock.uptimeMillis() < deadline) {
                        handler.postDelayed(this, 50)
                    }
                }
            }
            handler.post(poll)
        }
        assertTrue("native loading screen did not hand over to the WebView", painted.await(11, TimeUnit.SECONDS))
    }
}
