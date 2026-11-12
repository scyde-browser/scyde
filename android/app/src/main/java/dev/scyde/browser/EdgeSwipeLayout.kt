package dev.scyde.browser

import android.annotation.SuppressLint
import android.content.Context
import android.view.MotionEvent
import android.widget.FrameLayout
import kotlin.math.abs

/**
 * Hosts the GeckoView and watches for edge swipes: from the left edge → back, from the right
 * edge → forward (like Safari and Chrome). It only observes touches, so the page still gets them
 * (scrolling stays with Gecko). [onProgress] drives the arrow (side -1/+1, 0..1); 0 hides it.
 */
@SuppressLint("ViewConstructor")
class EdgeSwipeLayout(
    context: Context,
    /** Edge zone and commit distance in px, from the spacing tokens (rule 4). */
    private val edge: Float,
    private val distance: Float,
    private val onProgress: (side: Int, fraction: Float) -> Unit,
    private val onCommit: (side: Int) -> Unit,
) : FrameLayout(context) {
    private var side = 0
    private var startX = 0f
    private var startY = 0f
    private var fraction = 0f

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        when (ev.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                startX = ev.x
                startY = ev.y
                fraction = 0f
                side = when {
                    ev.x <= edge -> -1
                    ev.x >= width - edge -> 1
                    else -> 0
                }
            }
            MotionEvent.ACTION_MOVE -> if (side != 0) {
                val dx = (ev.x - startX) * -side // positive when moving away from the edge
                val dy = abs(ev.y - startY)
                if (dy > abs(dx) && dy > distance / 2) {
                    side = 0 // a vertical scroll, not a swipe
                    onProgress(0, 0f)
                } else {
                    fraction = (dx / distance).coerceIn(0f, 1f)
                    onProgress(side, fraction)
                }
            }
            MotionEvent.ACTION_UP -> {
                if (side != 0) {
                    if (fraction >= 1f) onCommit(side)
                    onProgress(0, 0f)
                }
                side = 0
            }
            MotionEvent.ACTION_CANCEL, MotionEvent.ACTION_POINTER_DOWN -> {
                if (side != 0) onProgress(0, 0f)
                side = 0
            }
        }
        return super.dispatchTouchEvent(ev)
    }
}
