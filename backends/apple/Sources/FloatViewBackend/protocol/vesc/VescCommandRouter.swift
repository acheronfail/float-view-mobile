import Foundation

public enum ControllerRoute: Sendable, Equatable {
  case direct
  case can(UInt8)

  public var canID: UInt8? {
    switch self {
    case .direct: nil
    case .can(let canID): canID
    }
  }
}

/// The sole owner of COMM_FORWARD_CAN wrapping.
public enum VescCommandRouter {
  public static func route(_ command: [UInt8], via route: ControllerRoute) -> [UInt8] {
    switch route {
    case .direct:
      return command
    case .can(let canID):
      precondition(canID <= 253)
      return [34, canID] + command
    }
  }
}
