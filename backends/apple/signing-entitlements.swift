// Read the signing system's decoded entitlements, including DER-only signatures.
import Foundation
import Security

func fail(_ message: String) -> Never {
  FileHandle.standardError.write(Data((message + "\n").utf8))
  exit(1)
}

guard CommandLine.arguments.count == 2 else {
  fail("Usage: signing-entitlements.swift <app-or-extension>")
}
var code: SecStaticCode?
let status = SecStaticCodeCreateWithPath(
  URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL, [], &code)
guard status == errSecSuccess, let code else { fail("Cannot open signed code: \(status)") }
var information: CFDictionary?
let result = SecCodeCopySigningInformation(
  code, SecCSFlags(rawValue: kSecCSSigningInformation), &information)
guard result == errSecSuccess, let dictionary = information as? [String: Any],
      let entitlements = dictionary[kSecCodeInfoEntitlementsDict as String] as? [String: Any] else {
  fail("Cannot read signed entitlements: \(result)")
}
do {
  FileHandle.standardOutput.write(try PropertyListSerialization.data(
    fromPropertyList: entitlements, format: .xml, options: 0))
} catch {
  fail("Cannot encode signed entitlements: \(error)")
}
