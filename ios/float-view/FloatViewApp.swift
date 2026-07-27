import SwiftUI

func print(_ items: Any..., separator: String = " ", terminator: String = "\n") {
  let output = items.map { "\($0)" }.joined(separator: separator)
  NSLog("%@", output)
}

@main
struct FloatViewApp: App {

  private let vescManager: VescBle
  private let vescCommands: VescCommandGenerator

  init() {
    LocalServer.shared.start()

    vescManager = VescBle()
    vescCommands = VescCommandGenerator()
    // TODO: don't hard-code
    vescCommands.setCanId(1)

    setupWebSocketHandler()
  }

  var body: some Scene {
    WindowGroup {
      ContentView(server: LocalServer.shared).preferredColorScheme(.dark)
    }
  }

  // MARK: - WebSocket Handler Setup (similar to MainActivity.kt)

  private func setupWebSocketHandler() {
    LocalServer.shared.setHandler { [vescManager] (cmd: BackendCommand) in
      switch cmd {
        case .scan: vescManager.startScan()
        case .connect(let address): vescManager.connect(address: address)
      case .disconnect( _): vescManager.disconnect()
        case .logStart(let address, let intervalMs):
          print("TODO: logStart for \(address) with interval \(intervalMs)ms")
        case .logStop(let address):
          print("TODO: logStop for \(address)")
      case .command( _, let cmd):
          switch (cmd) {
            case .commFwVersion:
              vescManager.send(vescCommands.firmware())
            case .commGetValues:
              vescManager.send(vescCommands.commGetValues())
          case .commGetValuesSetup:
              vescManager.send(vescCommands.commGetValuesSetup())
          case .commCustomAppData( _, let appCommand):
              switch appCommand {
                case .commandInfo(let version):
                  vescManager.send(vescCommands.refloatInfo(version: Int(version)))
                case .commandGetRtdata:
                  vescManager.send(vescCommands.refloatRtdata())
              case .commandGetAlldata(let mode):
                vescManager.send(vescCommands.refloatGetAlldata(mode))
              }
          }
      }
    }
  }

}
