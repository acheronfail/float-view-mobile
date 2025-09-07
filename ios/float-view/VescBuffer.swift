import Foundation

class VescBuffer {
    private let buffer: Data
    var index: Int = 0

    var size: Int {
        return buffer.count
    }

    init(data: Data) {
        self.buffer = data
    }

    func raw() -> Data {
        return buffer
    }

    func slice(expected: Int) -> Data {
        if index + expected <= buffer.count {
            let value = buffer.subdata(in: index..<index + expected - 1)
            index += expected - 1
            return value
        } else {
            return Data()
        }
    }

    func readUInt8Pair() -> (UInt8, UInt8) {
        let byte = readUInt8()
        let upper = (byte >> 4) & 0x0F  // Upper 4 bits
        let lower = byte & 0x0F         // Lower 4 bits
        return (upper, lower)
    }

    func readUInt8() -> UInt8 {
        guard index < buffer.count else { return 0 }
        let value = buffer[index]
        index += 1
        return value
    }

    func readInt8() -> Int8 {
        guard index < buffer.count else { return 0 }
        let value = Int8(bitPattern: buffer[index])
        index += 1
        return value
    }

    func readUInt16() -> UInt16 {
        guard index + 1 < buffer.count else { return 0 }
        let value = (UInt16(buffer[index]) << 8) | UInt16(buffer[index + 1])
        index += 2
        return value
    }

    func readInt16() -> Int16 {
        guard index + 1 < buffer.count else { return 0 }
        let value = (Int16(buffer[index]) << 8) | Int16(buffer[index + 1])
        index += 2
        return value
    }

    func readUInt32() -> UInt32 {
        guard index + 3 < buffer.count else { return 0 }
        let value = (UInt32(buffer[index]) << 24) |
                   (UInt32(buffer[index + 1]) << 16) |
                   (UInt32(buffer[index + 2]) << 8) |
                   UInt32(buffer[index + 3])
        index += 4
        return value
    }

    func readInt32() -> Int32 {
        guard index + 3 < buffer.count else { return 0 }
        let value = (Int32(buffer[index]) << 24) |
                   (Int32(buffer[index + 1]) << 16) |
                   (Int32(buffer[index + 2]) << 8) |
                   Int32(buffer[index + 3])
        index += 4
        return value
    }

    func readDouble16(scale: Int) -> Double {
        return Double(readInt16()) / Double(scale)
    }

    func readDouble32(scale: Int) -> Double {
        return Double(readInt32()) / Double(scale)
    }

    func readFloat32() -> Float {
        guard index + 3 < buffer.count else { return 0.0 }
        let bytes = buffer.subdata(in: index..<index + 4)
        index += 4
        return bytes.withUnsafeBytes {
            Float(bitPattern: $0.load(as: UInt32.self).bigEndian)
        }
    }

    func readString() -> String {
        var result = ""
        while index < buffer.count {
            let byte = readUInt8()
            if byte == 0x00 { break }
            result.append(Character(UnicodeScalar(byte)))
        }
        return result
    }

    // Writers

    func writeUInt8(_ value: UInt8) {
        // This would require a mutable buffer in a real implementation
        // For now, we'll focus on reading functionality
        index += 1
    }

    func writeInt8(_ value: Int8) {
        index += 1
    }

    func writeUInt16(_ value: UInt16) {
        index += 2
    }

    func writeInt16(_ value: Int16) {
        index += 2
    }

    func writeUInt32(_ value: UInt32) {
        index += 4
    }

    func writeInt32(_ value: Int32) {
        index += 4
    }

    func writeDouble16(_ value: Double, scale: Int) {
        let scaled = Int16(value * Double(scale))
        writeInt16(scaled)
    }

    func writeDouble32(_ value: Double, scale: Int) {
        let scaled = Int32(value * Double(scale))
        writeInt32(scaled)
    }

    func writeFloat32(_ value: Float) {
        index += 4
    }

    func writeString(_ value: String, terminate: Bool = true) {
        for char in value {
            writeUInt8(UInt8(char.unicodeScalars.first!.value))
        }
        if terminate {
            writeUInt8(0) // Null terminator
        }
    }
}
