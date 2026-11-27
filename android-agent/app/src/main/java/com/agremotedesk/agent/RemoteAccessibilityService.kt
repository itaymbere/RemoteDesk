package com.agremotedesk.agent

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.graphics.Path
import android.view.accessibility.AccessibilityEvent
import org.json.JSONObject

class RemoteAccessibilityService : AccessibilityService() {

    companion object {
        var instance: RemoteAccessibilityService? = null
            private set
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
    }

    override fun onUnbind(intent: android.content.Intent?): Boolean {
        instance = null
        return super.onUnbind(intent)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {}
    override fun onInterrupt() {}

    fun handleInputJson(jsonStr: String) {
        val obj = JSONObject(jsonStr)
        val dm = resources.displayMetrics
        when (obj.optString("type")) {
            "mousedown", "mousemove" -> {
                val x = (obj.optDouble("x", 0.5) * dm.widthPixels).toFloat()
                val y = (obj.optDouble("y", 0.5) * dm.heightPixels).toFloat()
                tapAt(x, y)
            }
        }
    }

    private fun tapAt(x: Float, y: Float) {
        val path = Path().apply { moveTo(x, y) }
        val stroke = GestureDescription.StrokeDescription(path, 0, 50)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        dispatchGesture(gesture, null, null)
    }
}
