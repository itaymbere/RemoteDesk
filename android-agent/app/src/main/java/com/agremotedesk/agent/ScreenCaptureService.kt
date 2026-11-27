package com.agremotedesk.agent

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.media.projection.MediaProjection
import android.os.IBinder
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import org.webrtc.DefaultVideoDecoderFactory
import org.webrtc.DefaultVideoEncoderFactory
import org.webrtc.EglBase
import org.webrtc.MediaConstraints
import org.webrtc.PeerConnection
import org.webrtc.PeerConnectionFactory
import org.webrtc.ScreenCapturerAndroid
import org.webrtc.SurfaceTextureHelper

class ScreenCaptureService : Service() {

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val http = OkHttpClient()

    private val serverUrl = "https://seu-servidor.com/api.php"
    private val hostId = "android-host-01"
    private val hostToken = "troque-este-token"

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        startForegroundNotification()
        val resultCode = intent?.getIntExtra("resultCode", 0) ?: 0
        val data = intent?.getParcelableExtra<Intent>("data")

        if (data != null) {
            scope.launch {
                registerHost()
                // Inicializa WebRTC PeerConnectionFactory + ScreenCapturerAndroid(data, callback)
                val eglBase = EglBase.create()
                PeerConnectionFactory.initialize(
                    PeerConnectionFactory.InitializationOptions.builder(this@ScreenCaptureService)
                        .createInitializationOptions()
                )
                val factory = PeerConnectionFactory.builder()
                    .setVideoEncoderFactory(DefaultVideoEncoderFactory(eglBase.eglBaseContext, true, true))
                    .setVideoDecoderFactory(DefaultVideoDecoderFactory(eglBase.eglBaseContext))
                    .createPeerConnectionFactory()

                val capturer = ScreenCapturerAndroid(data, object : MediaProjection.Callback() {})
                val helper = SurfaceTextureHelper.create("CaptureThread", eglBase.eglBaseContext)
                val source = factory.createVideoSource(capturer.isScreencast)
                capturer.initialize(helper, this@ScreenCaptureService, source.capturerObserver)
                capturer.startCapture(720, 1280, 30)

                val track = factory.createVideoTrack("video", source)
                val rtcConfig = PeerConnection.RTCConfiguration(
                    listOf(PeerConnection.IceServer.builder("stun:stun.l.google.com:19302").createIceServer())
                )

                var activePc: PeerConnection? = null
                var currentClientId = ""

                fun ensurePeerConnection(clientId: String): PeerConnection? {
                    currentClientId = clientId
                    activePc?.close()
                    activePc = factory.createPeerConnection(rtcConfig, object : PeerConnection.Observer {
                        override fun onSignalingChange(p0: PeerConnection.SignalingState?) {}
                        override fun onIceConnectionChange(p0: PeerConnection.IceConnectionState?) {}
                        override fun onIceConnectionReceivingChange(p0: Boolean) {}
                        override fun onIceGatheringChange(p0: PeerConnection.IceGatheringState?) {}
                        override fun onIceCandidate(cand: org.webrtc.IceCandidate?) {
                            if (cand != null && currentClientId.isNotEmpty()) {
                                val payload = JSONObject()
                                    .put("candidate", cand.sdp)
                                    .put("sdpMid", cand.sdpMid)
                                    .put("sdpMLineIndex", cand.sdpMLineIndex)
                                sendSignal(currentClientId, "ice", payload)
                            }
                        }
                        override fun onIceCandidatesRemoved(p0: Array<out org.webrtc.IceCandidate>?) {}
                        override fun onAddStream(p0: org.webrtc.MediaStream?) {}
                        override fun onRemoveStream(p0: org.webrtc.MediaStream?) {}
                        override fun onDataChannel(dc: org.webrtc.DataChannel?) {
                            dc?.registerObserver(object : org.webrtc.DataChannel.Observer {
                                override fun onBufferedAmountChange(p0: Long) {}
                                override fun onStateChange() {}
                                override fun onMessage(buf: org.webrtc.DataChannel.Buffer) {
                                    val bytes = ByteArray(buf.data.remaining())
                                    buf.data.get(bytes)
                                    RemoteAccessibilityService.instance?.handleInputJson(String(bytes))
                                }
                            })
                        }
                        override fun onRenegotiationNeeded() {}
                    })
                    activePc?.addTrack(track)
                    return activePc
                }

                // Loop SSE (api.php?action=poll)
                var lastId = 0L
                while (true) {
                    val req = Request.Builder()
                        .url("$serverUrl?action=poll&host_id=$hostId&token=$hostToken&last_id=$lastId")
                        .build()
                    runCatching {
                        http.newCall(req).execute().use { resp ->
                            val sourceBuf = resp.body?.source() ?: return@use
                            while (!sourceBuf.exhausted()) {
                                val line = sourceBuf.readUtf8Line() ?: break
                                if (line.startsWith("data: ")) {
                                    val msg = JSONObject(line.removePrefix("data: "))
                                    lastId = maxOf(lastId, msg.optLong("id", lastId))
                                    val senderId = msg.optString("sender_id")
                                    val type = msg.optString("type")
                                    val payload = msg.optJSONObject("payload") ?: continue

                                    when (type) {
                                        "offer" -> {
                                            val pc = ensurePeerConnection(senderId) ?: continue
                                            val sdp = org.webrtc.SessionDescription(
                                                org.webrtc.SessionDescription.Type.OFFER,
                                                payload.optString("sdp")
                                            )
                                            pc.setRemoteDescription(object : org.webrtc.SdpObserver {
                                                override fun onSetSuccess() {
                                                    pc.createAnswer(object : org.webrtc.SdpObserver {
                                                        override fun onCreateSuccess(ans: org.webrtc.SessionDescription) {
                                                            pc.setLocalDescription(this, ans)
                                                            val ansJson = JSONObject()
                                                                .put("type", "answer")
                                                                .put("sdp", ans.description)
                                                            sendSignal(senderId, "answer", ansJson)
                                                        }
                                                        override fun onSetSuccess() {}
                                                        override fun onCreateFailure(p0: String?) {}
                                                        override fun onSetFailure(p0: String?) {}
                                                    }, MediaConstraints())
                                                }
                                                override fun onCreateSuccess(p0: org.webrtc.SessionDescription?) {}
                                                override fun onCreateFailure(p0: String?) {}
                                                override fun onSetFailure(p0: String?) {}
                                            }, sdp)
                                        }
                                        "ice" -> {
                                            val cand = org.webrtc.IceCandidate(
                                                payload.optString("sdpMid"),
                                                payload.optInt("sdpMLineIndex"),
                                                payload.optString("candidate")
                                            )
                                            activePc?.addIceCandidate(cand)
                                        }
                                    }
                                }
                            }
                        }
                    }
                    kotlinx.coroutines.delay(2000)
                }
            }
        }

        return START_STICKY
    }

    private fun sendSignal(targetId: String, type: String, payload: JSONObject) {
        val json = JSONObject()
            .put("host_id", hostId)
            .put("token", hostToken)
            .put("target_id", targetId)
            .put("type", type)
            .put("payload", payload)
            .toString()

        val req = Request.Builder()
            .url("$serverUrl?action=signal")
            .post(json.toRequestBody("application/json".toMediaType()))
            .build()

        runCatching { http.newCall(req).execute().close() }
    }

    private fun registerHost() {
        val json = JSONObject()
            .put("host_id", hostId)
            .put("name", "Android Host")
            .put("os", "Android")
            .put("token", hostToken)
            .toString()

        val req = Request.Builder()
            .url("$serverUrl?action=host_register")
            .post(json.toRequestBody("application/json".toMediaType()))
            .build()

        runCatching { http.newCall(req).execute().close() }
    }

    private fun startForegroundNotification() {
        val channelId = "ag_remotedesk_capture"
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(
            NotificationChannel(channelId, "Captura de Tela", NotificationManager.IMPORTANCE_LOW)
        )
        val notification = Notification.Builder(this, channelId)
            .setContentTitle("Ag-RemoteDesk Agent")
            .setContentText("Compartilhamento de tela ativo")
            .setSmallIcon(android.R.drawable.ic_menu_camera)
            .build()
        startForeground(1, notification)
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }
}
