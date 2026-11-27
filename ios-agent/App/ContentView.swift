import SwiftUI
import ReplayKit

struct BroadcastPickerRepresentable: UIViewRepresentable {
    func makeUIView(context: Context) -> RPSystemBroadcastPickerView {
        let picker = RPSystemBroadcastPickerView(frame: CGRect(x: 0, y: 0, width: 60, height: 60))
        picker.showsMicrophoneButton = false
        return picker
    }

    func updateUIView(_ uiView: RPSystemBroadcastPickerView, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        VStack(spacing: 20) {
            Text("Ag-RemoteDesk iOS")
                .font(.title2.bold())
            Text("Toque abaixo para iniciar o compartilhamento de tela")
                .font(.subheadline)
                .foregroundColor(.secondary)
            BroadcastPickerRepresentable()
                .frame(width: 60, height: 60)
        }
        .padding()
    }
}
