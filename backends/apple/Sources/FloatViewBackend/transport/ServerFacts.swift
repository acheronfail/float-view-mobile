import Foundation

/// Encodes internal immutable domain facts; Rust owns the frontend projection.
enum ServerFacts {
  static func encode(_ value: Any) throws -> Any {
    if value is NSNull {
      return NSNull()
    }
    if let value = value as? DeviceID {
      return value.value
    }
    if let value = value as? Date {
      return ISO8601DateFormatter().string(from: value)
    }
    if let value = value as? String {
      return value
    }
    if let value = value as? NSNumber {
      return value
    }
    if let value = value as? ConnectionState {
      switch value {
      case .discoveringServices: return "discovering_services"
      case .enablingNotifications: return "enabling_notifications"
      default: return value.rawValue
      }
    }
    if let value = value as? FirmwareRecord {
      return ["major": value.major, "minor": value.minor,
              "hardwareName": value.hardwareName as Any? ?? NSNull()] as [String: Any]
    }
    if let value = value as? RefloatConfigurationValue {
      switch value {
      case let .double(number): return ["type": "double", "value": number]
      case let .integer(number): return ["type": "integer", "value": number]
      case let .boolean(boolean): return ["type": "boolean", "value": boolean]
      case let .string(string): return ["type": "string", "value": string]
      }
    }
    if let value = value as? ControllerRoute {
      switch value {
      case .direct: return ["kind": "direct"]
      case let .can(id): return ["kind": "can", "canId": id]
      }
    }
    if let value = value as? BmsConnection {
      switch value {
      case .direct: return ["kind": "direct"]
      case let .can(id): return ["kind": "can", "canId": id]
      }
    }
    if let value = value as? any RawRepresentable {
      return try encode(value.rawValue)
    }
    let mirror = Mirror(reflecting: value)
    switch mirror.displayStyle {
    case .optional:
      guard let child = mirror.children.first else { return NSNull() }
      return try encode(child.value)
    case .collection, .set: return try mirror.children.map { try encode($0.value) }
    case .dictionary:
      var result: [String: Any] = [:]
      for child in mirror.children {
        let entry = Array(Mirror(reflecting: child.value).children)
        guard entry.count == 2, let key = entry[0].value as? String else { throw ServerFactError.invalid }
        result[key] = try encode(entry[1].value)
      }
      return result
    case .struct:
      var result: [String: Any] = [:]
      for child in mirror.children {
        guard let label = child.label else { throw ServerFactError.invalid }
        var name = label.replacingOccurrences(of: "JSON", with: "Json").replacingOccurrences(of: "ID", with: "Id")
          .replacingOccurrences(
            of: "IC",
            with: "Ic"
          )
        if value is KnownDevice, label == "id" {
          name = "deviceId"
        }
        if value is KnownDevice || value is ActiveConnectionSnapshot, label == "advertisedName" {
          name = "name"
        }
        if value is RefloatConfigurationField, label == "type" {
          name = "fieldType"
        }
        result[name] = try encode(child.value)
      }
      return result
    default: throw ServerFactError.invalid
    }
  }
}

enum ServerFactError: Error { case invalid }
