package dev.scyde.browser

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.unit.dp

/**
 * Line icons, 1.5 px on a 24 grid (DESIGN.md §7; Lucide-style, ISC, paths redrawn), the same set as
 * design/prototypes/_shared/proto.js. Tinted by `Icon(tint = …)`.
 */
object ScydeIcons {
    private fun line(name: String, vararg paths: String, filled: Boolean = false): ImageVector =
        ImageVector.Builder(name, 24.dp, 24.dp, 24f, 24f).apply {
            for (d in paths) {
                addPath(
                    PathParser().parsePathString(d).toNodes(),
                    fill = if (filled) SolidColor(Color.Black) else null,
                    stroke = SolidColor(Color.Black),
                    strokeLineWidth = 1.5f,
                    strokeLineCap = StrokeCap.Round,
                    strokeLineJoin = StrokeJoin.Round,
                )
            }
        }.build()

    val back = line("back", "M12 19l-7-7 7-7", "M19 12H5")
    val forward = line("forward", "M5 12h14", "M12 5l7 7-7 7")
    val reload = line("reload", "M21 12a9 9 0 1 1-9-9c2.5 0 4.9 1 6.7 2.7L21 8", "M21 3v5h-5")
    val close = line("close", "M18 6L6 18", "M6 6l12 12")
    val share = line("share", "M4 12v7a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-7", "M16 6l-4-4-4 4", "M12 2v13")
    val plus = line("plus", "M5 12h14", "M12 5v14")
    val incognito = line(
        "incognito",
        "M9.88 9.88a3 3 0 1 0 4.24 4.24",
        "M10.73 5.08A10.4 10.4 0 0 1 12 5c7 0 10 7 10 7a13.2 13.2 0 0 1-1.67 2.68",
        "M6.61 6.61A13.5 13.5 0 0 0 2 12s3 7 10 7a9.7 9.7 0 0 0 5.39-1.61",
        "M2 2l20 20",
    )
    private const val STAR = "M12 3l2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.3l-5.6 2.9 1.1-6.2L3 9.6l6.2-.9z"
    val star = line("star", STAR)
    val starFilled = line("starFilled", STAR, filled = true)
    val find = line("find", "M16 10a6 6 0 1 1-12 0a6 6 0 1 1 12 0", "M20 20l-5.5-5.5")
    val desktop = line("desktop", "M4 3h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z", "M8 21h8", "M12 17v4")
    val copy = line("copy", "M10 8h9a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-9a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2z", "M4 16V5a1 1 0 0 1 1-1h11")
    val history = line("history", "M21 12a9 9 0 1 1-18 0a9 9 0 1 1 18 0", "M12 7v5l3 2")
    val book = line("book", "M4 19.5A2.5 2.5 0 0 1 6.5 17H20", "M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z")
    val settings = line(
        "settings",
        "M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1",
        "M17 6a2 2 0 1 1-4 0a2 2 0 1 1 4 0",
        "M11 12a2 2 0 1 1-4 0a2 2 0 1 1 4 0",
        "M19 18a2 2 0 1 1-4 0a2 2 0 1 1 4 0",
    )
    val lock = line("lock", "M6 11h12a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2z", "M8 11V7a4 4 0 0 1 8 0v4")
    val search = line("search", "M18 11a7 7 0 1 1-14 0a7 7 0 1 1 14 0", "M20 20l-3.5-3.5")
    val up = line("up", "M18 15l-6-6-6 6")
    val down = line("down", "M6 9l6 6 6-6")
    val chevron = line("chevron", "M9 18l6-6-6-6")
    val tabs = line("tabs", "M8 4h10a2 2 0 0 1 2 2v10", "M6 8h10a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z")
    val shield = line(
        "shield",
        "M20 13c0 5-3.5 7.5-7.7 9a1 1 0 0 1-.6 0C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.2-2.7a1.2 1.2 0 0 1 1.6 0C14.5 3.8 17 5 19 5a1 1 0 0 1 1 1z",
    )
    val open = line("open", "M15 3h6v6", "M10 14L21 3", "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6")
}

/**
 * The SCYDE mark (brand/icon/scyde.svg): two strokes, the second rotated 180° around the centre
 * (263.5, 284), fitted into a `size` square. The union spans x 68–459 and y 82–486.
 */
fun brandMarkPath(size: Float): Path {
    val cx = 263.5f
    val cy = 284f
    val scale = size / 404f
    val path = Path()
    for (flip in listOf(false, true)) {
        fun x(v: Float) = ((if (flip) 2 * cx - v else v) - cx) * scale + size / 2
        fun y(v: Float) = ((if (flip) 2 * cy - v else v) - cy) * scale + size / 2
        path.moveTo(x(400f), y(82f))
        path.lineTo(x(251f), y(82f))
        path.cubicTo(x(110f), y(82f), x(68f), y(101f), x(68f), y(241f))
        path.cubicTo(x(68f), y(328f), x(160f), y(382f), x(160f), y(382f))
        path.lineTo(x(243f), y(308.5f))
        path.lineTo(x(214f), y(242f))
        path.close()
    }
    return path
}
