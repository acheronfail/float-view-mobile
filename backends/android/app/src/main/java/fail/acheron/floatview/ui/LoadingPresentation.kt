package fail.acheron.floatview.ui

import android.content.Context
import android.graphics.Color
import android.view.Gravity
import android.widget.Button
import android.widget.ScrollView
import android.widget.ImageView
import fail.acheron.floatview.application.StartupFailure
import android.widget.LinearLayout
import android.widget.TextView
import fail.acheron.floatview.R

/** Native placeholder until the early HTML loading screen has painted. */
class LoadingPresentation(context: Context) : LinearLayout(context) {
    private var dark = false
    private var failed = false
    private val symbol = ImageView(context).apply {
        setImageResource(R.drawable.loading_symbol)
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    private val message = TextView(context).apply {
        text = "Loading…"
        textSize = 15f
        gravity = Gravity.CENTER
    }

    private val detail = TextView(context).apply {
        textSize = 16f
        gravity = Gravity.CENTER
        visibility = GONE
    }
    private val retry = Button(context).apply {
        text = "Try again"
        isAllCaps = false
        visibility = GONE
    }

    init {
        orientation = VERTICAL
        val density = resources.displayMetrics.density
        val spacing = (24 * density).toInt()
        val column = LinearLayout(context).apply {
            orientation = VERTICAL
            gravity = Gravity.CENTER
            setPadding(spacing, spacing * 2, spacing, spacing * 2)
        }
        column.addView(symbol, LayoutParams((72 * density).toInt(), (96 * density).toInt()))
        for (view in listOf(message, detail, retry)) {
            column.addView(view, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT).apply {
                topMargin = (20 * density).toInt()
                gravity = Gravity.CENTER_HORIZONTAL
            })
        }
        addView(ScrollView(context).apply {
            isFillViewport = true
            addView(column)
        }, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        applyTheme(false)
    }

    fun applyTheme(dark: Boolean) {
        this.dark = dark
        symbol.setColorFilter(Color.parseColor(if (dark) "#2DD4BF" else "#14B8A6"))
        detail.setTextColor(Color.parseColor(if (dark) "#CBD5E1" else "#334155"))
        message.setTextColor(Color.parseColor(
            if (failed) { if (dark) "#E2E8F0" else "#0F172A" }
            else { if (dark) "#94A3B8" else "#64748B" },
        ))
    }

    fun showLoading() {
        failed = false
        applyTheme(dark)
        message.text = "Loading…"
        message.textSize = 15f
        message.setTypeface(null, android.graphics.Typeface.NORMAL)
        message.isAccessibilityHeading = false
        detail.visibility = GONE
        retry.visibility = GONE
        retry.setOnClickListener(null)
    }
    fun showFailure(failure: StartupFailure, onRetry: () -> Unit) {
        failed = true
        applyTheme(dark)
        message.text = failure.title
        message.textSize = 22f
        message.setTypeface(null, android.graphics.Typeface.BOLD)
        message.isAccessibilityHeading = true
        message.accessibilityLiveRegion = ACCESSIBILITY_LIVE_REGION_POLITE
        detail.text = failure.recovery
        detail.visibility = VISIBLE
        retry.visibility = VISIBLE
        retry.setOnClickListener { onRetry() }
    }
}
