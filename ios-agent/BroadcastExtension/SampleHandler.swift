import ReplayKit
import WebRTC

class SampleHandler: RPBroadcastSampleHandler {

    private let serverUrl = "https://seu-servidor.com/api.php"
    private let hostId = "ios-host-01"
    private let hostToken = "troque-este-token"

    private var factory: RTCPeerConnectionFactory?
    private var videoSource: RTCVideoSource?
    private var peerConnection: RTCPeerConnection?
    private var currentClientId: String = ""
    private var lastSignalId: Int = 0
    private var isRunning = false

    override func broadcastStarted(withSetupInfo setupInfo: [String: NSObject]?) {
        isRunning = true
        registerHost()
        RTCInitializeSSL()
        factory = RTCPeerConnectionFactory()
        videoSource = factory?.videoSource()
        Task { await pollSseLoop() }
    }

    override func broadcastFinished() {
        isRunning = false
        peerConnection?.close()
        peerConnection = nil
        factory = nil
        videoSource = nil
        RTCCleanupSSL()
    }

    override func processSampleBuffer(_ sampleBuffer: CMSampleBuffer, with sampleBufferType: RPSampleBufferType) {
        guard sampleBufferType == .video,
              let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer),
              let videoSource = videoSource else { return }

        let timeStampNs = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sampleBuffer)) * 1_000_000_000
        let rtcPixelBuffer = RTCCVPixelBuffer(pixelBuffer: pixelBuffer)
        let videoFrame = RTCVideoFrame(buffer: rtcPixelBuffer, rotation: ._0, timeStampNs: Int64(timeStampNs))
        videoSource.capturer(RTCVideoCapturer(), didCapture: videoFrame)
    }

    private func registerHost() {
        guard let url = URL(string: "\(serverUrl)?action=host_register") else { return }
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let body: [String: String] = [
            "host_id": hostId,
            "name": "iOS Host",
            "os": "iOS",
            "token": hostToken,
        ]
        req.httpBody = try? JSONSerialization.data(withJSONObject: body)
        URLSession.shared.dataTask(with: req).resume()
    }

    private func sendSignal(targetId: String, type: String, payload: [String: Any]) {
        guard let url = URL(string: "\(serverUrl)?action=signal") else { return }
        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let body: [String: Any] = [
            "host_id": hostId,
            "token": hostToken,
            "target_id": targetId,
            "type": type,
            "payload": payload,
        ]
        req.httpBody = try? JSONSerialization.data(withJSONObject: body)
        URLSession.shared.dataTask(with: req).resume()
    }

    private func pollSseLoop() async {
        while isRunning {
            guard let url = URL(string: "\(serverUrl)?action=poll&host_id=\(hostId)&token=\(hostToken)&last_id=\(lastSignalId)") else { return }
            do {
                let (bytes, _) = try await URLSession.shared.bytes(from: url)
                for try await line in bytes.lines {
                    guard line.hasPrefix("data: ") else { continue }
                    let jsonStr = String(line.dropFirst(6))
                    guard let data = jsonStr.data(using: .utf8),
                          let msg = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                          let senderId = msg["sender_id"] as? String,
                          let type = msg["type"] as? String,
                          let payload = msg["payload"] as? [String: Any] else { continue }

                    if let id = msg["id"] as? Int {
                        lastSignalId = max(lastSignalId, id)
                    }

                    if type == "offer", let sdpStr = payload["sdp"] as? String {
                        handleOffer(from: senderId, sdp: sdpStr)
                    } else if type == "ice", let candStr = payload["candidate"] as? String {
                        let sdpMid = payload["sdpMid"] as? String
                        let sdpMLineIndex = payload["sdpMLineIndex"] as? Int32 ?? 0
                        let cand = RTCIceCandidate(sdp: candStr, sdpMLineIndex: sdpMLineIndex, sdpMid: sdpMid)
                        peerConnection?.add(cand) { _ in }
                    }
                }
            } catch {
                try? await Task.sleep(nanoseconds: 2_000_000_000)
            }
        }
    }

    private func handleOffer(from clientId: String, sdp: String) {
        guard let factory = factory, let videoSource = videoSource else { return }
        currentClientId = clientId
        peerConnection?.close()

        let config = RTCConfiguration()
        config.iceServers = [RTCIceServer(urlStrings: ["stun:stun.l.google.com:19302"])]
        let constraints = RTCMediaConstraints(mandatoryConstraints: nil, optionalConstraints: nil)
        let pc = factory.peerConnection(with: config, constraints: constraints, delegate: nil)
        let track = factory.videoTrack(with: videoSource, trackId: "video")
        pc?.add(track, streamIds: ["ag-remotedesk"])
        peerConnection = pc

        let remoteSdp = RTCSessionDescription(type: .offer, sdp: sdp)
        pc?.setRemoteDescription(remoteSdp) { [weak self] _ in
            pc?.answer(for: constraints) { answer, _ in
                guard let answer = answer else { return }
                pc?.setLocalDescription(answer) { _ in
                    self?.sendSignal(targetId: clientId, type: "answer", payload: [
                        "type": "answer",
                        "sdp": answer.sdp,
                    ])
                }
            }
        }
    }
}
