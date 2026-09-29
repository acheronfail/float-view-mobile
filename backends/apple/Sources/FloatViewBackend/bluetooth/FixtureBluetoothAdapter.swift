#if FLOAT_VIEW_CONFORMANCE && DEBUG && (os(macOS) || targetEnvironment(simulator))
  import Foundation

  /// Test transport only. Device identity, events and protocol bytes belong to conformance.
  public final class FixtureBluetoothAdapter: BluetoothPort, @unchecked Sendable {
    private let event: @Sendable (BluetoothEvent) -> Void
    private let endpoint: URL
    private let token: String
    private let run: String
    private let sessionID: String
    private let queue = DispatchQueue(label: "float-view.fixture-relay")
    private let lock = NSLock()
    private var pending = 0
    private var pendingBytes = 0
    private var failed = false
    private var serviceGeneration = BluetoothConnection(lifetime: 0, generation: 0)
    private var desiredGeneration = 0
    private var operation = 0
    private var sequence = 0
    private var device: DeviceID?

    public init(eventHandler: @escaping @Sendable (BluetoothEvent) -> Void) {
      let env = ProcessInfo.processInfo.environment
      guard let raw = env["FLOAT_VIEW_FIXTURE_ENDPOINT"], let url = URL(string: raw),
            url.scheme == "http", url.host == "127.0.0.1", url.port != nil,
            url.path.isEmpty || url.path == "/", url.user == nil, url.password == nil,
            url.query == nil, url.fragment == nil,
            let token = env["FLOAT_VIEW_FIXTURE_TOKEN"], token.count == 48,
            let run = env["FLOAT_VIEW_FIXTURE_RUN"], run.count == 48,
            let session = env["FLOAT_VIEW_FIXTURE_SESSION"], session.count == 48
      else { fatalError("Missing or invalid conformance fixture credentials") }
      endpoint = url; self.token = token; self.run = run; sessionID = session; event = eventHandler
      heartbeat()
    }

    public func currentState() -> BluetoothState {
      BluetoothState(radio: .poweredOn, scanPermission: .granted, connectPermission: .granted)
    }

    public func requestPermission() {
      event(.stateChanged(currentState()))
    }

    public func startDiscovery() {
      submit("scan")
    }

    public func stopDiscovery() {
      submit("stopScan")
    }

    public func connect(_ deviceID: DeviceID, generation: BluetoothConnection) {
      submit("connect", device: deviceID, serviceGeneration: generation)
    }

    public func disconnect(_ deviceID: DeviceID, generation: BluetoothConnection) {
      guard lock.withLock({ serviceGeneration == generation }) else { return }
      submit("disconnect", device: deviceID, serviceGeneration: generation)
    }

    public func enableNotifications(generation: BluetoothConnection, operation: UInt64) {
      submit("enableNotifications", serviceGeneration: generation, transportOperation: operation)
    }

    public func write(
      _ bytes: Data,
      to deviceID: DeviceID,
      generation: BluetoothConnection,
      operation: UInt64,
      response: Bool
    ) {
      guard bytes.count <= 8192 else { fail(); return }
      submit(
        "write",
        device: deviceID,
        bytes: bytes,
        serviceGeneration: generation,
        transportOperation: operation,
        response: response
      )
    }

    private func heartbeat() {
      queue.asyncAfter(deadline: .now() + .milliseconds(250)) { [weak self] in
        guard let self, !lock.withLock({ self.failed }) else { return }
        submit("poll")
        heartbeat()
      }
    }

    private func submit(
      _ command: String,
      device: DeviceID? = nil,
      bytes: Data? = nil,
      serviceGeneration: BluetoothConnection? = nil,
      transportOperation: UInt64? = nil,
      response: Bool = false
    ) {
      lock.lock()
      let count = bytes?.count ?? 0
      guard !failed, pending < 64, pendingBytes + count <= 65536 else { lock.unlock(); fail(); return }
      pending += 1
      pendingBytes += count
      if command == "connect" {
        desiredGeneration += 1
      }
      if let serviceGeneration {
        self.serviceGeneration = serviceGeneration
      }
      let capturedGeneration = self.serviceGeneration
      let generation = desiredGeneration
      if let device {
        self.device = device
      }
      let deadline = DispatchTime.now().uptimeNanoseconds + 4_500_000_000
      // Admission and enqueue share a lock, preserving caller order.
      queue.async { [weak self] in
        guard let self else { return }
        defer { lock.withLock { self.pending -= 1; self.pendingBytes -= count } }
        guard !lock.withLock({ self.failed }) else { return }
        operation += 1
        var body: [String: Any] = ["run": run, "session": sessionID, "operation": operation,
                                   "generation": generation, "command": command]
        if let device {
          body["device"] = device.value
        }
        body["transport_operation"] = transportOperation
        body["response"] = response
        if let bytes {
          body["bytes"] = bytes.base64EncodedString()
        }
        do {
          let now = DispatchTime.now().uptimeNanoseconds
          guard now < deadline else { throw URLError(.timedOut) }
          let remaining = min(4, Double(deadline - now) / 1_000_000_000)
          var request = URLRequest(url: endpoint, timeoutInterval: remaining)
          request.httpMethod = "POST"
          request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
          request.setValue(sessionID, forHTTPHeaderField: "X-Fixture-Session")
          request.httpBody = try JSONSerialization.data(withJSONObject: body)
          let data = try FixtureHTTP.receive(request, timeout: remaining)
          guard let reply = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                reply["run"] as? String == run, reply["session"] as? String == sessionID,
                reply["operation"] as? Int == operation, reply["generation"] as? Int == generation,
                let events = reply["events"] as? [[String: Any]], events.count <= 128
          else { throw URLError(.badServerResponse) }
          for value in events {
            guard let next = value["sequence"] as? Int, next > sequence,
                  let eventGeneration = value["generation"] as? Int,
                  let sourceOperation = value["operation"] as? Int, sourceOperation > 0, sourceOperation <= operation,
                  let rawID = value["device"] as? String, rawID.utf8.count <= 256
            else { throw URLError(.badServerResponse) }
            sequence = next
            guard eventGeneration == lock.withLock({ self.desiredGeneration }) else { continue }
            let id = DeviceID(rawID)
            switch value["event"] as? String {
            case "discovered": event(.deviceFound(
                id,
                name: value["name"] as? String,
                rssi: value["rssi"] as? Int,
                uart: value["uart"] as? Bool == true
              ))
            case "profile":
              guard let profile = value["profile"] as? [String: Any], let service = profile["service"] as? Bool,
                    let notify = profile["notify"] as? Bool, let withResponse = profile["with_response"] as? Int,
                    let withoutResponse = profile["without_response"] as? Int
              else { throw URLError(.badServerResponse) }
              event(.profile(
                capturedGeneration,
                BluetoothProfile(
                  service: service,
                  notify: notify,
                  withResponse: withResponse,
                  withoutResponse: withoutResponse
                )
              ))
            case "notifications":
              guard let op = value["transport_operation"] as? UInt64,
                    let success = value["success"] as? Bool else { throw URLError(.badServerResponse) }
              event(.notifications(capturedGeneration, op, success))
            case "writeProgress":
              guard let op = value["transport_operation"] as? UInt64, let progress = value["progress"] as? String,
                    ["submitted", "acknowledged", "available"].contains(progress)
              else { throw URLError(.badServerResponse) }
              event(.writeProgress(capturedGeneration, op, progress))
            case "link":
              guard let raw = value["state"] as? String, let state = ConnectionState(rawValue: raw) else {
                throw URLError(.badServerResponse)
              }
              event(.connectionChanged(id, capturedGeneration, state))
            case "bytes":
              guard let raw = value["bytes"] as? String, raw.count <= 10924,
                    let bytes = Data(base64Encoded: raw), bytes.count <= 8192
              else { throw URLError(.badServerResponse) }
              event(.notification(id, capturedGeneration, bytes))
            default: throw URLError(.badServerResponse)
            }
          }
        } catch { fail() }
      }
      lock.unlock()
    }

    private func fail() {
      let result = lock.withLock { () -> (Bool, DeviceID?) in
        let first = !failed; failed = true; return (first, device)
      }
      guard result.0 else { return }
      event(.scanFailed("Conformance fixture relay failed"))
      if let device = result.1 {
        event(.connectionChanged(device, lock.withLock { serviceGeneration }, .disconnected))
      }
    }
  }

  private final class FixtureHTTP: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    private var bytes = Data()
    private var error: Error?
    private let done = DispatchSemaphore(value: 0)
    static func receive(_ request: URLRequest, timeout: TimeInterval) throws -> Data {
      let receiver = FixtureHTTP()
      let configuration = URLSessionConfiguration.ephemeral
      configuration.timeoutIntervalForResource = timeout
      let session = URLSession(configuration: configuration, delegate: receiver, delegateQueue: nil)
      defer { session.invalidateAndCancel() }
      session.dataTask(with: request).resume()
      guard receiver.done.wait(timeout: .now() + timeout) == .success else { throw URLError(.timedOut) }
      if let error = receiver.error {
        throw error
      }
      return receiver.bytes
    }

    func urlSession(_: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
      guard bytes.count + data.count <= 65536
      else { error = URLError(.dataLengthExceedsMaximum); dataTask.cancel(); return }
      bytes.append(data)
    }

    func urlSession(_: URLSession, task: URLSessionTask, didCompleteWithError failure: Error?) {
      if failure != nil {
        error = failure
      }
      if (task.response as? HTTPURLResponse)?.statusCode != 200 {
        error = URLError(.badServerResponse)
      }
      done.signal()
    }

    func urlSession(_: URLSession, task _: URLSessionTask, willPerformHTTPRedirection _: HTTPURLResponse,
                    newRequest _: URLRequest, completionHandler: @escaping @Sendable (URLRequest?) -> Void)
    {
      completionHandler(nil)
    }
  }
#endif
