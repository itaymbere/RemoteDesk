import ReplayKit
import WebRTC

class SampleHandler: RPBroadcastSampleHandler {

    private let serverUrl = "https://seu-servidor.com/api.php"
    private let hostId = "ios-host-01"
    private let hostToken = "troque-este-token"

    private var factory: RTCPeerConnectionFactory?
    private var videoSource: RTCVideoSource?

    override func broadcastStarted(withSetupInfo setupInfo: [String: NSObject]?) {
        registerHost()
        RTCInitializeSSL()
        factory = RTCPeerConnectionFactory()
        videoSource = factory?.videoSource()
        // TODO: Conectar loop SSE (api.php?action=poll) e criar RTCPeerConnection com videoSource
    }

    override func broadcastFinished() {
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
}
