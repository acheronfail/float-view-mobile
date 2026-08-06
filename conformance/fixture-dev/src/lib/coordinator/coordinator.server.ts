import { composeInteractiveBoard } from '../../../../fixture/boards.ts';
import { chunks } from '../../../../fixture/runner.ts';
import { led, type FixtureControl, type FixtureEvent } from '../../../../fixture/serial.ts';
import { VescSimulator, type FixtureRequestAttempt, type RecordedRequest } from '../../../../fixture/simulator.ts';
import {
  encodeVbms32FirmwarePayload,
  InteractiveBmsSource,
  typicalBmsValues,
  type BmsConfiguration,
} from '../../../../fixture/bms.ts';
import {
  idleTelemetry,
  InteractiveTelemetrySource,
  telemetryScenes,
  type TelemetrySample,
} from '../../../../fixture/telemetry.ts';
import {
  firmwareOptions,
  refloatOptions,
  topologyOptions,
  type FixtureAction,
  type FixtureConfiguration,
  type LastFixtureAttempt,
  type FixtureSnapshot,
  type SerialPortOption,
} from '../contract.ts';
import {
  availableTelemetryFields,
  defaultBmsNode,
  parseScenarioDocument,
  privateBmsIdentity,
  privateControllerIdentity,
  scenarioTemplate,
} from './scenario-document.server.ts';

const MAX_DIAGNOSTICS = 50;
const MAX_PORTS = 32;
const READY_TIMEOUT_MS = 5_000;

export interface CoordinatorDependencies {
  open(path: string): Promise<FixtureControl>;
  listPorts(): Promise<SerialPortOption[]>;
  log?: (level: FixtureLogLevel, message: string) => void;
}

export type FixtureLogLevel = 'info' | 'warn' | 'error';

export class FixtureActionError extends Error {
  readonly snapshot: FixtureSnapshot;

  constructor(message: string, snapshot: FixtureSnapshot) {
    super(message);
    this.snapshot = snapshot;
  }
}

export class FixtureCoordinator {
  private readonly dependencies: CoordinatorDependencies;
  private readonly logger: NonNullable<CoordinatorDependencies['log']>;
  private fixture?: FixtureControl;
  private simulator: VescSimulator;
  private telemetry = new InteractiveTelemetrySource();
  private replacementSequence = 0;
  private queue: Promise<unknown> = Promise.resolve();
  private loopGeneration = 0;
  private readyWaiter?: { resolve: () => void; reject: (error: Error) => void; timer: NodeJS.Timeout };
  private state: FixtureSnapshot = {
    revision: 0,
    serial: { status: 'detached' },
    configuration: {
      topology: 'express',
      canId: 1,
      firmware: '6.06',
      refloat: '1.2.3',
      bmsTopology: 'none',
      bmsCanId: 42,
      bmsVariant: 'extended',
      bmsFailureMode: 'none',
      bmsCellCount: 18,
      bmsTemperatureCount: 4,
      bmsStateOfCharge: 0.8,
      bmsInputCurrentA: -4,
    },
    scenario: scenarioTemplate('singleExpress'),
    frozen: false,
    pinnedFields: {},
    availableFields: availableTelemetryFields('1.2.3'),
    power: 'off',
    ble: 'unavailable',
    ride: 'idle',
    counters: { writes: 0, responses: 0, telemetry: 0, bms: 0, dropped: 0 },
    diagnostics: [],
  };

  constructor(dependencies: CoordinatorDependencies) {
    this.dependencies = dependencies;
    this.logger = dependencies.log ?? terminalLogger;
    this.simulator = this.createSimulator();
  }

  snapshot(): FixtureSnapshot {
    return structuredClone(this.state);
  }

  async ports(): Promise<SerialPortOption[]> {
    const ports = await this.dependencies.listPorts();
    return ports.slice(0, MAX_PORTS).map(({ path, label }) => ({
      path: path.slice(0, 512),
      label: label.slice(0, 160),
    }));
  }

  act(action: FixtureAction): Promise<FixtureSnapshot> {
    return this.enqueue(async () => {
      this.clearError();
      switch (action.kind) {
        case 'attach':
          await this.attach(action.path);
          break;
        case 'detach':
          await this.detach();
          break;
        case 'configure':
          this.configure(action.configuration);
          break;
        case 'power-on':
          await this.powerOn();
          break;
        case 'power-off':
          await this.powerOff();
          break;
        case 'start-riding':
          this.startRiding();
          break;
        case 'stop-riding':
          this.stopRiding();
          break;
        case 'apply-template':
          this.applyScenario(scenarioTemplate(action.template));
          break;
        case 'import-scenario':
          this.applyScenario(parseScenarioDocument(action.document));
          break;
        case 'set-scene':
          this.setScene(action.scene);
          break;
        case 'freeze':
          this.setFrozen(true);
          break;
        case 'resume':
          this.setFrozen(false);
          break;
        case 'pin-field':
          this.pinField(action.field, action.value);
          break;
        case 'unpin-field':
          this.telemetry.unpin(action.field);
          this.syncTelemetryState();
          break;
        case 'reset-overrides':
          this.telemetry.reset();
          this.state.scenario.overrides = {};
          this.syncTelemetryState();
          break;
        case 'disconnect-readvertise':
          await this.disconnectAndReadvertise();
          break;
        case 'reconnect-unchanged':
          await this.disconnectAndReadvertise();
          break;
        case 'move-controller':
          await this.mutateConnectionScenario((draft) => {
            const controller = this.controller(action.alias, draft);
            if (draft.controllers.some((item) => item.canId === action.canId)) {
              this.rejectAction('Controller CAN ID is already in use');
            }
            controller.canId = action.canId;
          });
          break;
        case 'replace-controller':
          await this.mutateConnectionScenario((draft) => {
            const controller = this.controller(action.alias, draft);
            controller.alias = `replacement-${++this.replacementSequence}`;
          });
          break;
        case 'set-ping-visible':
          await this.mutateConnectionScenario((draft) => {
            this.controller(action.alias, draft).pingVisible = action.visible;
          });
          break;
        case 'set-fragment-bytes':
          if (!Number.isInteger(action.fragmentBytes) || action.fragmentBytes < 1 || action.fragmentBytes > 244) {
            this.rejectAction('Notification fragment bound must be from 1 through 244');
          }
          this.state.scenario.fragmentBytes = action.fragmentBytes;
          this.update({ scenario: structuredClone(this.state.scenario) });
          break;
      }
      return this.snapshot();
    });
  }

  async initialise(initialPath?: string): Promise<void> {
    if (!initialPath) return;
    await this.act({ kind: 'attach', path: initialPath }).catch(() => undefined);
  }

  async cleanup(): Promise<void> {
    await this.enqueue(async () => {
      const fixture = this.fixture;
      this.fixture = undefined;
      this.loopGeneration += 1;
      this.rejectReady(new Error('Fixture coordinator stopped'));
      if (fixture) {
        await fixture.command('RESET').catch(() => undefined);
        await fixture.led(led.off).catch(() => undefined);
        await fixture.close().catch(() => undefined);
      }
      this.telemetry.stop();
      this.update({
        serial: { status: 'detached' },
        power: 'off',
        ble: 'unavailable',
        ride: 'idle',
      });
    });
  }

  private enqueue<T>(operation: () => Promise<T> | T): Promise<T> {
    const result = this.queue.then(operation, operation);
    this.queue = result.catch(() => undefined);
    return result;
  }

  private async attach(path: string) {
    if (this.fixture) this.rejectAction('Detach the current fixture before attaching another port');
    if (!path || path.length > 512) this.rejectAction('Select a valid bounded serial path');
    this.update({ serial: { status: 'attaching', path }, ble: 'unavailable' });
    this.log('info', 'serial attaching');
    let fixture: FixtureControl | undefined;
    try {
      fixture = await this.dependencies.open(path);
      await this.resetBeforeLoop(fixture);
      this.fixture = fixture;
      this.loopGeneration += 1;
      this.telemetry.stop();
      this.update({
        serial: { status: 'attached', path },
        power: 'off',
        ble: 'idle',
        ride: 'idle',
      });
      this.log('info', 'serial attached');
      void this.runLoop(fixture, this.loopGeneration);
    } catch (error) {
      await fixture?.close().catch(() => undefined);
      this.fixture = undefined;
      this.update({
        serial: { status: 'detached', error: message(error) },
        power: 'off',
        ble: 'unavailable',
        ride: 'idle',
      });
      this.diagnostic(`Serial attachment failed: ${observableMessage(error)}`);
      this.log('error', `serial attachment failed: ${observableMessage(error)}`);
      throw new FixtureActionError(message(error), this.snapshot());
    }
  }

  private async detach() {
    const fixture = this.fixture;
    if (!fixture) return;
    await fixture.command('RESET').catch(() => undefined);
    await fixture.led(led.off).catch(() => undefined);
    this.fixture = undefined;
    this.loopGeneration += 1;
    this.rejectReady(new Error('Fixture detached'));
    await fixture.close().catch(() => undefined);
    this.telemetry.stop();
    this.update({
      serial: { status: 'detached' },
      power: 'off',
      ble: 'unavailable',
      ride: 'idle',
    });
    this.log('info', 'serial detached by user');
  }

  private configure(configuration: FixtureConfiguration) {
    if (this.state.power !== 'off' || this.state.ble === 'advertising' || this.state.ble === 'connected') {
      this.rejectAction('Power off and disconnect before saving fixture configuration');
    }
    try {
      validateConfiguration(configuration);
    } catch (error) {
      this.rejectAction(message(error));
    }
    const next = scenarioTemplate(configuration.topology === 'direct' ? 'direct' : 'singleExpress');
    next.controllers[0] = {
      ...next.controllers[0]!,
      canId: configuration.canId,
      firmware: configuration.firmware,
      refloat: configuration.refloat,
    };
    if (configuration.bmsTopology !== 'none') {
      const node = defaultBmsNode(configuration.bmsTopology === 'direct' ? 'direct' : 'can', configuration.bmsCanId);
      node.variant = configuration.bmsVariant;
      node.failureMode = configuration.bmsFailureMode;
      node.cellCount = configuration.bmsCellCount;
      node.temperatureCount = configuration.bmsTemperatureCount;
      node.stateOfCharge = configuration.bmsStateOfCharge;
      node.inputCurrentA = configuration.bmsInputCurrentA;
      next.bmsNodes = [node];
      if (configuration.bmsTopology === 'direct') {
        next.topology = 'express';
        next.controllers[0]!.pingVisible = true;
      }
    }
    this.applyScenario(next);
  }

  private applyScenario(scenario: ReturnType<typeof parseScenarioDocument> | ReturnType<typeof scenarioTemplate>) {
    if (this.state.power !== 'off' || this.state.ble === 'advertising' || this.state.ble === 'connected') {
      this.rejectAction('Power off and disconnect before replacing the fixture scenario');
    }
    const validated = parseScenarioDocument(scenario);
    this.telemetry = new InteractiveTelemetrySource();
    this.telemetry.setScene(validated.scene);
    for (const [field, value] of Object.entries(validated.overrides)) {
      this.telemetry.pin(field as keyof TelemetrySample, value as never);
    }
    this.state = {
      ...this.state,
      revision: this.state.revision + 1,
      configuration: configurationFrom(validated),
      scenario: structuredClone(validated),
      ride: this.telemetry.phase(),
      telemetry: this.telemetry.lastSample(),
      frozen: false,
      pinnedFields: this.telemetry.pinnedFields(),
      availableFields: this.resolvedAvailableFields(validated),
      lastAttempt: undefined,
      lastRequest: undefined,
      counters: { writes: 0, responses: 0, telemetry: 0, bms: 0, dropped: 0 },
      diagnostics: [],
    };
    this.simulator = this.createSimulator();
  }

  private async powerOn() {
    const fixture = this.requireFixture();
    if (this.state.power === 'on') return;
    await fixture.led(led.advertising);
    await fixture.command('ADVERTISE');
    this.update({ power: 'on', ble: 'advertising' });
    this.log('info', 'power on; BLE advertising');
  }

  private async powerOff() {
    const fixture = this.requireFixture();
    if (this.state.power === 'off') return;
    const ready = this.waitForReady();
    await fixture.command('RESET');
    await ready;
    this.telemetry.stop();
    await fixture.led(led.off).catch(() => undefined);
    this.update({ power: 'off', ble: 'idle', ride: 'idle', telemetry: idleTelemetrySnapshot() });
    this.log('info', 'power off');
  }

  private startRiding() {
    this.requireFixture();
    if (this.state.power !== 'on') this.rejectAction('Power on before starting a ride');
    if (!this.telemetry.start()) return;
    this.state.scenario.scene = this.telemetry.currentScene();
    this.syncTelemetryState();
    this.log('info', 'ride started');
  }

  private stopRiding() {
    this.requireFixture();
    if (this.state.power !== 'on') this.rejectAction('Power on before stopping a ride');
    if (!this.telemetry.stop()) return;
    this.state.scenario.scene = 'idle';
    this.syncTelemetryState();
    this.log('info', 'ride stopped');
  }

  private setScene(scene: (typeof telemetryScenes)[number]) {
    if (!telemetryScenes.includes(scene)) this.rejectAction('Unknown telemetry scene');
    this.telemetry.setScene(scene);
    this.state.scenario.scene = scene;
    this.state.scenario.overrides = { ...this.telemetry.pinnedFields() };
    this.syncTelemetryState();
  }

  private setFrozen(frozen: boolean) {
    const changed = frozen ? this.telemetry.freeze() : this.telemetry.resume();
    if (changed) this.syncTelemetryState();
  }

  private pinField(field: keyof TelemetrySample, value: TelemetrySample[keyof TelemetrySample]) {
    this.telemetry.pin(field, value as never);
    this.state.scenario.overrides = { ...this.telemetry.pinnedFields() };
    this.syncTelemetryState();
  }

  private syncTelemetryState() {
    this.update({
      scenario: structuredClone(this.state.scenario),
      ride: this.telemetry.phase(),
      telemetry: this.telemetry.lastSample(),
      frozen: this.telemetry.isFrozen(),
      pinnedFields: this.telemetry.pinnedFields(),
    });
  }

  private controller(alias: string, scenario = this.state.scenario) {
    const controller = scenario.controllers.find((item) => item.alias === alias);
    if (!controller) this.rejectAction('Controller alias was not found');
    return controller;
  }

  private async mutateConnectionScenario(mutation: (draft: FixtureSnapshot['scenario']) => void) {
    const draft = structuredClone(this.state.scenario);
    mutation(draft);
    const validated = parseScenarioDocument(draft);
    const active = this.state.power === 'on';
    if (active) await this.requireFixture().command('DISCONNECT');
    this.state.scenario = validated;
    this.simulator = this.createSimulator();
    if (active) await this.requireFixture().command('ADVERTISE');
    this.update({
      scenario: structuredClone(validated),
      configuration: configurationFrom(validated),
      availableFields: this.resolvedAvailableFields(validated),
      ble: active ? 'advertising' : this.state.ble,
    });
  }

  private async disconnectAndReadvertise() {
    const fixture = this.requireFixture();
    if (this.state.power !== 'on') this.rejectAction('Power on before reconnecting');
    await fixture.command('DISCONNECT');
    await fixture.command('ADVERTISE');
    this.update({ ble: 'advertising' });
  }

  private async runLoop(fixture: FixtureControl, generation: number) {
    while (this.fixture === fixture && this.loopGeneration === generation) {
      try {
        const event = await fixture.next(1_000);
        if (event.kind === 'READY') {
          this.resolveReady();
          continue;
        }
        await this.enqueue(() => this.handleEvent(fixture, event));
      } catch (error) {
        if (this.fixture !== fixture || this.loopGeneration !== generation) return;
        if (message(error).startsWith('Timed out waiting for')) continue;
        await this.enqueue(() => this.handleLoss(fixture, error));
        return;
      }
    }
  }

  private async handleEvent(fixture: FixtureControl, event: FixtureEvent) {
    if (this.fixture !== fixture) return;
    switch (event.kind) {
      case 'READY':
        this.resolveReady();
        break;
      case 'CONNECTED':
        await fixture.led(led.running).catch(() => undefined);
        this.update({ ble: 'connected' });
        this.log('info', 'BLE connected');
        break;
      case 'DISCONNECTED':
        this.log('info', 'BLE disconnected');
        if (this.state.power === 'on') {
          await fixture.led(led.advertising).catch(() => undefined);
          await fixture.command('ADVERTISE');
          this.update({ ble: 'advertising' });
        } else {
          this.update({ ble: 'idle' });
        }
        break;
      case 'ERROR':
        if (event.code === 'NOT_CONNECTED' && this.state.ble !== 'connected') {
          this.diagnostic('Late notification ignored after disconnect');
          break;
        }
        throw new Error(`Fixture error: ${event.code}`);
      case 'WRITE': {
        this.increment('writes');
        const droppedBefore = this.simulator.diagnostics.length;
        let responses;
        try {
          responses = this.simulator.receive(event.bytes, (attempt) => this.beginAttempt(attempt));
        } catch (error) {
          this.failRequest('simulate', error);
          break;
        }
        if (this.simulator.diagnostics.length > droppedBefore) {
          this.failRequest('simulate', this.simulator.diagnostics.at(-1) ?? 'Request dropped');
        }
        for (const response of responses) {
          this.ensureAttempt(response.request);
          this.updateAttempt({ stage: 'fragment', outcome: 'pending', error: undefined });
          let fragments: Uint8Array[];
          try {
            fragments = chunks(response.frame, this.state.scenario.fragmentBytes);
          } catch (error) {
            this.failRequest('fragment', error);
            continue;
          }
          this.updateAttempt({ stage: 'notify', outcome: 'pending', error: undefined });
          try {
            for (const fragment of fragments) await fixture.notify(fragment);
          } catch (error) {
            this.updateAttempt({ stage: 'notify', outcome: 'failed', error: bounded(message(error)) });
            throw error;
          }
          this.increment('responses');
          this.update({
            lastAttempt: {
              ...this.attemptFromRequest(response.request),
              stage: 'notify',
              outcome: 'responded',
            },
            lastRequest: {
              kind: response.request.kind,
              command: response.request.command,
              ...(response.request.canId === undefined ? {} : { canId: response.request.canId }),
              ...(response.request.telemetryCommand === undefined
                ? {}
                : { telemetryCommand: response.request.telemetryCommand }),
              responseBytes: response.frame.byteLength,
              fragments: fragments.length,
            },
          });
          if (response.request.kind === 'telemetry') {
            this.increment('telemetry');
            this.update({ telemetry: this.telemetry.lastSample() });
          }
          if (response.request.kind === 'bms-values') this.increment('bms');
        }
        break;
      }
    }
  }

  private async handleLoss(fixture: FixtureControl, error: unknown) {
    if (this.fixture !== fixture) return;
    this.fixture = undefined;
    this.loopGeneration += 1;
    this.rejectReady(new Error(message(error)));
    await fixture.close().catch(() => undefined);
    this.telemetry.stop();
    const reason = observableMessage(error);
    this.update({
      serial: { status: 'detached', error: reason },
      power: 'off',
      ble: 'unavailable',
      ride: 'idle',
    });
    this.diagnostic(`Serial detached: ${reason}`);
    this.log('error', `serial detached: ${reason}`);
  }

  private async resetBeforeLoop(fixture: FixtureControl) {
    const deadline = Date.now() + READY_TIMEOUT_MS;
    while (Date.now() < deadline) {
      await fixture.command('RESET');
      try {
        const event = await fixture.next(Math.min(500, Math.max(1, deadline - Date.now())));
        if (event.kind === 'READY') return;
      } catch (error) {
        if (!message(error).startsWith('Timed out waiting for')) throw error;
      }
    }
    throw new Error('Fixture did not acknowledge RESET with READY');
  }

  private waitForReady(timeoutMs = READY_TIMEOUT_MS): Promise<void> {
    if (this.readyWaiter) throw new Error('Fixture READY wait is already active');
    return new Promise((resolve, reject) => {
      this.readyWaiter = {
        resolve,
        reject,
        timer: setTimeout(() => {
          this.readyWaiter = undefined;
          reject(new Error('Timed out waiting for fixture READY'));
        }, timeoutMs),
      };
    });
  }

  private resolveReady() {
    if (!this.readyWaiter) return;
    clearTimeout(this.readyWaiter.timer);
    const resolve = this.readyWaiter.resolve;
    this.readyWaiter = undefined;
    resolve();
  }

  private rejectReady(error: Error) {
    if (!this.readyWaiter) return;
    clearTimeout(this.readyWaiter.timer);
    const reject = this.readyWaiter.reject;
    this.readyWaiter = undefined;
    reject(error);
  }

  private createSimulator() {
    const bmsConfigurations = this.state.scenario.bmsNodes.map((node): BmsConfiguration => {
      const source = new InteractiveBmsSource({
        seed: node.seed,
        cellCount: node.cellCount,
        temperatureCount: node.temperatureCount,
        stateOfCharge: node.stateOfCharge,
        inputCurrentA: node.inputCurrentA,
        reportedCanId: node.canId,
      });
      return {
        ...(node.route === 'can' ? { canId: node.canId } : {}),
        firmware: {
          major: 6,
          minor: 6,
          hardwareName: 'VBMS32',
          hardwareType: 2,
          payload: encodeVbms32FirmwarePayload(
            Uint8Array.from(privateBmsIdentity(this.state.scenario, node.alias).slice(0, 6))
          ),
        },
        values: { ...typicalBmsValues, reportedCanId: node.canId },
        valuesSource: source,
        tail:
          node.variant === 'legacy' ? { kind: 'legacy' } : { kind: 'extended', dataVersion: 1, status: node.status },
        pingVisible: node.pingVisible,
        failureMode: node.failureMode,
      };
    });
    const localBms = bmsConfigurations.find((_, index) => this.state.scenario.bmsNodes[index]?.route === 'direct');
    const canBms = bmsConfigurations.filter((_, index) => this.state.scenario.bmsNodes[index]?.route === 'can');
    return new VescSimulator(
      composeInteractiveBoard({
        advertisedName: this.state.scenario.advertisedName,
        topology: this.state.scenario.topology,
        controllers: this.state.scenario.controllers.map((item) => ({
          ...item,
          uuid: privateControllerIdentity(this.state.scenario, item.alias),
        })),
        canPingIds: [
          ...this.state.scenario.discoveredCanIds,
          ...(this.state.scenario.topology === 'express'
            ? this.state.scenario.controllers.filter((item) => item.pingVisible).map((item) => item.canId)
            : []),
          ...this.state.scenario.bmsNodes
            .filter((item) => item.route === 'can' && item.pingVisible)
            .map((item) => item.canId),
        ],
        telemetry: this.telemetry,
        localBms,
        bmsNodes: canBms,
      })
    );
  }

  private resolvedAvailableFields(scenario = this.state.scenario) {
    const catalogs = scenario.controllers.map((item) => new Set(availableTelemetryFields(item.refloat)));
    return availableTelemetryFields(scenario.controllers[0]!.refloat).filter((field) =>
      catalogs.every((catalog) => catalog.has(field))
    );
  }

  private requireFixture() {
    if (!this.fixture) this.rejectAction('Attach a fixture serial port first');
    return this.fixture;
  }

  private rejectAction(reason: string): never {
    this.update({ serial: { ...this.state.serial, error: reason } });
    throw new FixtureActionError(reason, this.snapshot());
  }

  private clearError() {
    if (!this.state.serial.error) return;
    this.update({ serial: { ...this.state.serial, error: undefined } });
  }

  private update(patch: Partial<FixtureSnapshot>) {
    this.state = {
      ...this.state,
      ...patch,
      revision: this.state.revision + 1,
    };
  }

  private increment(counter: keyof FixtureSnapshot['counters']) {
    this.update({
      counters: {
        ...this.state.counters,
        [counter]: this.state.counters[counter] + 1,
      },
    });
  }

  private beginAttempt(attempt: FixtureRequestAttempt) {
    this.update({
      lastAttempt: {
        ...attempt,
        stage: 'simulate',
        outcome: 'pending',
      },
    });
    if (!attempt.telemetryCommand) this.log('info', `request ${requestLabel(attempt)}`);
  }

  private ensureAttempt(request: RecordedRequest) {
    const attempt = this.attemptFromRequest(request);
    const current = this.state.lastAttempt;
    if (current?.command === attempt.command && current.canId === attempt.canId) return;
    this.beginAttempt(attempt);
  }

  private attemptFromRequest(request: RecordedRequest): FixtureRequestAttempt {
    return {
      command: request.command,
      ...(request.canId === undefined ? {} : { canId: request.canId }),
      ...(request.telemetryCommand === undefined ? {} : { telemetryCommand: request.telemetryCommand }),
    };
  }

  private updateAttempt(patch: Pick<LastFixtureAttempt, 'stage' | 'outcome'> & { error?: string }) {
    const current = this.state.lastAttempt;
    if (!current) return;
    this.update({ lastAttempt: { ...current, ...patch } });
  }

  private failRequest(stage: LastFixtureAttempt['stage'], error: unknown) {
    const reason = observableMessage(error);
    const current = this.state.lastAttempt?.outcome === 'pending' ? this.state.lastAttempt : undefined;
    const attempt: LastFixtureAttempt = {
      ...(current ?? { command: 255, stage, outcome: 'pending' as const }),
      stage,
      outcome: 'dropped',
      error: reason,
    };
    this.increment('dropped');
    this.update({ lastAttempt: attempt });
    const diagnostic = `Protocol request dropped during ${stage}: ${requestLabel(attempt)}: ${reason}`;
    this.diagnostic(diagnostic);
    this.log('warn', diagnostic);
  }

  private log(level: FixtureLogLevel, value: string) {
    this.logger(level, bounded(redact(value)));
  }

  private diagnostic(value: string) {
    const diagnostics = [...this.state.diagnostics, value.slice(0, 240)].slice(-MAX_DIAGNOSTICS);
    this.update({ diagnostics });
  }
}

function requestLabel(attempt: Pick<FixtureRequestAttempt, 'command' | 'canId'>) {
  return `command=${attempt.command} route=${attempt.canId === undefined ? 'direct' : `CAN:${attempt.canId}`}`;
}

function terminalLogger(level: FixtureLogLevel, message: string) {
  const line = `[fixture] ${message}`;
  if (level === 'error') console.error(line);
  else if (level === 'warn') console.warn(line);
  else console.info(line);
}

function bounded(value: string) {
  return value.slice(0, 240);
}

function observableMessage(error: unknown) {
  return bounded(redact(message(error)));
}

function redact(value: string) {
  return value
    .replace(/Invalid fixture event:.*/i, 'Invalid fixture event')
    .replace(/([?&]token=)[^&\s]+/gi, '$1[redacted]')
    .replace(/\b(?:[0-9a-f]{2}:){5}[0-9a-f]{2}\b/gi, '[address]')
    .replace(/\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/gi, '[identifier]')
    .replace(/\/dev\/[^\s]+/g, '[serial-port]')
    .replace(/\b[0-9a-f]{48,}\b/gi, '[frame omitted]');
}

function validateConfiguration(configuration: FixtureConfiguration) {
  if (!topologyOptions.includes(configuration.topology)) throw new Error('Invalid topology');
  if (!firmwareOptions.includes(configuration.firmware)) throw new Error('Invalid firmware');
  if (!refloatOptions.includes(configuration.refloat)) throw new Error('Invalid Refloat version');
  if (!Number.isInteger(configuration.canId) || configuration.canId < 0 || configuration.canId > 253) {
    throw new Error('CAN ID must be an integer from 0 through 253');
  }
  const scenario = scenarioTemplate(configuration.topology === 'direct' ? 'direct' : 'singleExpress');
  scenario.controllers[0]!.canId = configuration.canId;
  if (configuration.bmsTopology !== 'none') {
    const node = defaultBmsNode(configuration.bmsTopology === 'direct' ? 'direct' : 'can', configuration.bmsCanId);
    node.variant = configuration.bmsVariant;
    node.failureMode = configuration.bmsFailureMode;
    node.cellCount = configuration.bmsCellCount;
    node.temperatureCount = configuration.bmsTemperatureCount;
    node.stateOfCharge = configuration.bmsStateOfCharge;
    node.inputCurrentA = configuration.bmsInputCurrentA;
    scenario.bmsNodes = [node];
    if (configuration.bmsTopology === 'direct') scenario.topology = 'express';
  }
  parseScenarioDocument(scenario);
}

function configurationFrom(scenario: FixtureSnapshot['scenario']): FixtureConfiguration {
  const primary = scenario.controllers[0]!;
  const bms = scenario.bmsNodes[0];
  return {
    topology: scenario.topology,
    canId: primary.canId,
    firmware: primary.firmware,
    refloat: primary.refloat,
    bmsTopology: bms?.route ?? 'none',
    bmsCanId: bms?.canId ?? 42,
    bmsVariant: bms?.variant ?? 'extended',
    bmsFailureMode: bms?.failureMode ?? 'none',
    bmsCellCount: bms?.cellCount ?? 18,
    bmsTemperatureCount: bms?.temperatureCount ?? 4,
    bmsStateOfCharge: bms?.stateOfCharge ?? 0.8,
    bmsInputCurrentA: bms?.inputCurrentA ?? -4,
  };
}

function idleTelemetrySnapshot() {
  return { ...idleTelemetry };
}

function message(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
