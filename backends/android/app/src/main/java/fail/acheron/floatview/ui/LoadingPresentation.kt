package fail.acheron.floatview.ui

import android.content.Context
import android.graphics.Color
import android.view.Gravity
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import fail.acheron.floatview.R

/** Native placeholder until the early HTML loading screen has painted. */
class LoadingPresentation(context: Context) : LinearLayout(context) {
    private val symbol = ImageView(context).apply {
        setImageResource(R.drawable.loading_symbol)
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    private val message = TextView(context).apply {
        text = "Loading…"
        textSize = 15f
        gravity = Gravity.CENTER
    }

    init {
        orientation = VERTICAL
        gravity = Gravity.CENTER
        val density = resources.displayMetrics.density
        addView(symbol, LayoutParams((72 * density).toInt(), (96 * density).toInt()))
        addView(message, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT).apply {
            topMargin = (20 * density).toInt()
        })
        applyTheme(false)
    }

    fun applyTheme(dark: Boolean) {
        symbol.setColorFilter(Color.parseColor(if (dark) "#A2D4D8" else "#0F766E"))
        message.setTextColor(Color.parseColor(if (dark) "#94A3B8" else "#64748B"))
    }

    fun showLoading() { message.text = "Loading…" }
    fun showFailure() { message.text = "Unable to open Float View" }
}
