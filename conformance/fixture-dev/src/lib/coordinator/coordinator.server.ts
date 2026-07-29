import { composeBoard } from '../../../../fixture/boards.ts';
import { chunks } from '../../../../fixture/runner.ts';
import {
	led,
	type FixtureControl,
	type FixtureEvent
} from '../../../../fixture/serial.ts';
import { VescSimulator } from '../../../../fixture/simulator.ts';
import { InteractiveTelemetrySource } from '../../../../fixture/telemetry.ts';
import {
	firmwareOptions,
	refloatOptions,
	topologyOptions,
	type FixtureAction,
	type FixtureConfiguration,
	type FixtureSnapshot,
	type SerialPortOption
} from '../contract.ts';

const MAX_DIAGNOSTICS = 100;
const MAX_PORTS = 32;
const READY_TIMEOUT_MS = 5_000;

export interface CoordinatorDependencies {
	open(path: string): Promise<FixtureControl>;
	listPorts(): Promise<SerialPortOption[]>;
}

export class FixtureActionError extends Error {
	readonly snapshot: FixtureSnapshot;

	constructor(message: string, snapshot: FixtureSnapshot) {
		super(message);
		this.snapshot = snapshot;
	}
}

export class FixtureCoordinator {
	private readonly dependencies: CoordinatorDependencies;
	private fixture?: FixtureControl;
	private simulator: VescSimulator;
	private telemetry = new InteractiveTelemetrySource();
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
			refloat: '1.2.3'
		},
		power: 'off',
		ble: 'unavailable',
		ride: 'idle',
		counters: { writes: 0, responses: 0, telemetry: 0, dropped: 0 },
		diagnostics: []
	};

	constructor(dependencies: CoordinatorDependencies) {
		this.dependencies = dependencies;
		this.simulator = this.createSimulator();
	}

	snapshot(): FixtureSnapshot {
		return structuredClone(this.state);
	}

	async ports(): Promise<SerialPortOption[]> {
		const ports = await this.dependencies.listPorts();
		return ports.slice(0, MAX_PORTS).map(({ path, label }) => ({
			path: path.slice(0, 512),
			label: label.slice(0, 160)
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
				ride: 'idle'
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
				ride: 'idle'
			});
			void this.runLoop(fixture, this.loopGeneration);
		} catch (error) {
			await fixture?.close().catch(() => undefined);
			this.fixture = undefined;
			this.update({
				serial: { status: 'detached', error: message(error) },
				power: 'off',
				ble: 'unavailable',
				ride: 'idle'
			});
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
			ride: 'idle'
		});
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
		this.telemetry = new InteractiveTelemetrySource();
		this.state = {
			...this.state,
			revision: this.state.revision + 1,
			configuration: { ...configuration },
			ride: 'idle',
			telemetry: undefined,
			counters: { writes: 0, responses: 0, telemetry: 0, dropped: 0 },
			diagnostics: []
		};
		this.simulator = this.createSimulator();
	}

	private async powerOn() {
		const fixture = this.requireFixture();
		if (this.state.power === 'on') return;
		await fixture.led(led.advertising);
		await fixture.command('ADVERTISE');
		this.update({ power: 'on', ble: 'advertising' });
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
	}

	private startRiding() {
		this.requireFixture();
		if (this.state.power !== 'on') this.rejectAction('Power on before starting a ride');
		if (!this.telemetry.start()) return;
		this.update({ ride: 'riding' });
	}

	private stopRiding() {
		this.requireFixture();
		if (this.state.power !== 'on') this.rejectAction('Power on before stopping a ride');
		if (!this.telemetry.stop()) return;
		this.update({ ride: 'idle', telemetry: idleTelemetrySnapshot() });
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
				break;
			case 'DISCONNECTED':
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
				const responses = this.simulator.receive(event.bytes);
				if (this.simulator.diagnostics.length > droppedBefore) {
					this.increment('dropped');
					this.diagnostic(this.simulator.diagnostics.at(-1) ?? 'Request dropped');
				}
				for (const response of responses) {
					for (const fragment of chunks(response.frame, 244)) await fixture.notify(fragment);
					this.increment('responses');
					if (response.request.kind === 'telemetry') {
						this.increment('telemetry');
						this.update({ telemetry: this.telemetry.lastSample() });
					}
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
		this.update({
			serial: { status: 'detached', error: message(error) },
			power: 'off',
			ble: 'unavailable',
			ride: 'idle'
		});
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
				}, timeoutMs)
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
		return new VescSimulator(
			composeBoard({
				...this.state.configuration,
				telemetry: this.telemetry,
				requestPolicy: 'interactive'
			})
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
			revision: this.state.revision + 1
		};
	}

	private increment(counter: keyof FixtureSnapshot['counters']) {
		this.update({
			counters: {
				...this.state.counters,
				[counter]: this.state.counters[counter] + 1
			}
		});
	}

	private diagnostic(value: string) {
		const diagnostics = [...this.state.diagnostics, value.slice(0, 240)].slice(-MAX_DIAGNOSTICS);
		this.update({ diagnostics });
	}
}

function validateConfiguration(configuration: FixtureConfiguration) {
	if (!topologyOptions.includes(configuration.topology)) throw new Error('Invalid topology');
	if (!firmwareOptions.includes(configuration.firmware)) throw new Error('Invalid firmware');
	if (!refloatOptions.includes(configuration.refloat)) throw new Error('Invalid Refloat version');
	if (!Number.isInteger(configuration.canId) || configuration.canId < 0 || configuration.canId > 253) {
		throw new Error('CAN ID must be an integer from 0 through 253');
	}
}

function idleTelemetrySnapshot() {
	return {
		rideState: 'idle' as const,
		boardState: 0,
		footpadState: 0,
		speedKph: 0,
		erpm: 0,
		dutyCycle: 0,
		motorCurrentA: 0,
		batteryCurrentA: 0,
		packVoltageV: 63.4,
		controllerTempC: 30,
		motorTempC: 28
	};
}

function message(error: unknown) {
	return error instanceof Error ? error.message : String(error);
}
