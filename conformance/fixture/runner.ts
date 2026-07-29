import type { FixtureScenario } from './scenarios.ts';
import { led, type FixtureControl, type FixtureEvent } from './serial.ts';
import { VescSimulator, type RecordedRequest } from './simulator.ts';

const MAX_DIAGNOSTICS = 100;
export type FixtureProgress = (message: string) => void;

export class FixtureRunner {
	readonly scenario: FixtureScenario;
	readonly simulator: VescSimulator;
	private readonly fixture: FixtureControl;
	private readonly readyRetryMs: number;
	private readonly report: FixtureProgress;
	private running = false;
	private loop?: Promise<void>;
	private firstFailure?: Error;
	private injectedReconnect = false;
	private disconnectInjected = false;
	private connected = false;
	private telemetryResponses = 0;
	private telemetryPhase?: string;
	private readyWaiter?: {
		resolve: () => void;
		reject: (error: Error) => void;
		timer: NodeJS.Timeout;
	};
	private readonly diagnostics: string[] = [];
	private readonly waiters = new Set<{
		predicate: () => boolean;
		resolve: () => void;
		reject: (error: Error) => void;
		timer: NodeJS.Timeout;
	}>();

	constructor(
		scenario: FixtureScenario,
		fixture: FixtureControl,
		now?: () => number,
		readyRetryMs = 1_000,
		report: FixtureProgress = () => undefined
	) {
		this.scenario = scenario;
		this.fixture = fixture;
		this.readyRetryMs = readyRetryMs;
		this.report = report;
		this.simulator = new VescSimulator(scenario.board, now);
	}

	async start() {
		if (this.running) throw new Error('Fixture runner is already active');
		await this.resetAndWaitReady();
		await this.setLed(led.initializing, 'initializing');
		await this.setLed(led.advertising, 'advertising');
		await this.sendCommand('ADVERTISE');
		this.running = true;
		this.loop = this.runLoop();
	}

	async waitForRequests(
		predicate: (requests: readonly RecordedRequest[]) => boolean,
		timeoutMs = 15_000
	) {
		if (predicate(this.simulator.requests)) return;
		if (this.waiters.size >= 32) throw new Error('Too many fixture runner waits');
		await new Promise<void>((resolve, reject) => {
			const waiter = {
				predicate: () => predicate(this.simulator.requests),
				resolve,
				reject,
				timer: setTimeout(() => {
					this.waiters.delete(waiter);
					reject(new Error('Timed out waiting for VESC requests'));
				}, timeoutMs)
			};
			this.waiters.add(waiter);
		});
	}

	async injectDisconnectAndReadvertise() {
		this.injectedReconnect = true;
		this.disconnectInjected = true;
		await this.sendCommand('DISCONNECT');
	}

	verify() {
		if (this.firstFailure) throw this.firstFailure;
		const telemetry = this.simulator.requests.filter((item) => item.kind === 'telemetry');
		if (telemetry.length < this.scenario.minimumTelemetryResponses) {
			throw new Error(
				`${this.scenario.name} expected ${this.scenario.minimumTelemetryResponses} telemetry responses, observed ${telemetry.length}`
			);
		}
		if (
			this.scenario.expectedTelemetryIntervalMs !== undefined &&
			this.scenario.cadenceToleranceMs !== undefined
		) {
			this.simulator.verifyCadence(
				this.scenario.expectedTelemetryIntervalMs,
				this.scenario.cadenceToleranceMs
			);
		}
	}

	diagnosticSnapshot() {
		return [...this.diagnostics];
	}

	async stop(passed: boolean) {
		if (passed && !this.firstFailure) {
			await this.setLed(led.passed, 'passed').catch(() => undefined);
		} else {
			await this.setLed(led.failed, 'failed').catch(() => undefined);
		}
		let cleanupFailure: Error | undefined;
		try {
			if (this.running && !this.firstFailure) await this.resetAndConfirmReady();
			else await this.sendCommand('RESET');
		} catch (error) {
			cleanupFailure = error instanceof Error ? error : new Error(String(error));
		}
		this.running = false;
		await this.setLed(led.off, 'off').catch(() => undefined);
		await this.fixture.close().catch(() => undefined);
		await this.loop?.catch(() => undefined);
		this.rejectWaiters(new Error('Fixture runner stopped'));
		if (passed && cleanupFailure) throw cleanupFailure;
	}

	private async runLoop() {
		while (this.running) {
			try {
				const event = await this.fixture.next(60_000);
				await this.handle(event);
			} catch (error) {
				if (!this.running) return;
				await this.fail(error);
				return;
			}
		}
	}

	private async handle(event: FixtureEvent) {
		this.record(event.kind === 'WRITE' ? `WRITE ${Buffer.from(event.bytes).toString('hex')}` : event.kind);
		switch (event.kind) {
			case 'CONNECTED':
				this.report('ESP < CONNECTED');
				this.connected = true;
				await this.setLed(led.running, 'running');
				break;
			case 'DISCONNECTED':
				this.report('ESP < DISCONNECTED');
				this.connected = false;
				if (this.injectedReconnect) {
					this.injectedReconnect = false;
					this.simulator.resetTelemetry();
					await this.setLed(led.advertising, 'advertising');
					await this.sendCommand('ADVERTISE');
				}
				break;
			case 'ERROR':
				// A peer may disconnect after writing a request but before the
				// queued notification command reaches the fixture.
				if (event.code === 'NOT_CONNECTED' && !this.connected) {
					this.report('ESP < ERROR NOT_CONNECTED (late notification ignored)');
					break;
				}
				this.report(`ESP < ERROR ${event.code}`);
				throw new Error(`Fixture error: ${event.code}`);
			case 'WRITE': {
				const responses = this.simulator.receive(event.bytes);
				for (const response of responses) {
					const fragments = chunks(response.frame, this.scenario.fragmentBytes);
					this.reportResponse(
						response.request.kind,
						response.request.telemetryPhase,
						response.frame.length,
						fragments.length
					);
					for (const chunk of fragments) {
						await this.fixture.notify(chunk);
					}
					this.record(
						`RESPONSE ${response.request.kind}${response.request.telemetryPhase ? `:${response.request.telemetryPhase}` : ''}`
					);
					if (
						this.scenario.disconnectAfterTelemetry !== undefined &&
						!this.disconnectInjected &&
						this.simulator.requests.filter((item) => item.kind === 'telemetry').length ===
							this.scenario.disconnectAfterTelemetry
					) {
						await this.injectDisconnectAndReadvertise();
					}
				}
				break;
			}
			case 'READY':
				this.report('ESP < READY');
				this.resolveReadyWaiter();
				break;
		}
		this.resolveWaiters();
	}

	private async resetAndWaitReady(timeoutMs = 10_000) {
		const deadline = performance.now() + timeoutMs;
		while (performance.now() < deadline) {
			await this.sendCommand('RESET');
			try {
				const event = await this.fixture.next(
					Math.max(1, Math.min(this.readyRetryMs, deadline - performance.now()))
				);
				if (event.kind === 'READY') {
					this.report('ESP < READY');
					return;
				}
			} catch (error) {
				if (!(error instanceof Error) || !error.message.startsWith('Timed out waiting for')) {
					throw error;
				}
			}
		}
		throw new Error(
			'Fixture did not become ready after RESET; confirm the flashed firmware and serial port'
		);
	}

	private async fail(error: unknown) {
		this.firstFailure ??= error instanceof Error ? error : new Error(String(error));
		this.record(`FAIL ${this.firstFailure.message}`);
		this.rejectWaiters(this.firstFailure);
		this.rejectReadyWaiter(this.firstFailure);
		await this.setLed(led.failed, 'failed').catch(() => undefined);
	}

	private async resetAndConfirmReady(timeoutMs = 5_000) {
		if (this.readyWaiter) throw new Error('Fixture READY wait is already active');
		const ready = new Promise<void>((resolve, reject) => {
			this.readyWaiter = {
				resolve,
				reject,
				timer: setTimeout(() => {
					this.readyWaiter = undefined;
					reject(new Error('Timed out waiting for fixture RESET acknowledgement'));
				}, timeoutMs)
			};
		});
		try {
			await this.sendCommand('RESET');
			await ready;
		} catch (error) {
			this.rejectReadyWaiter(error instanceof Error ? error : new Error(String(error)));
			await ready.catch(() => undefined);
			throw error;
		}
	}

	private resolveReadyWaiter() {
		if (!this.readyWaiter) return;
		clearTimeout(this.readyWaiter.timer);
		const { resolve } = this.readyWaiter;
		this.readyWaiter = undefined;
		resolve();
	}

	private rejectReadyWaiter(error: Error) {
		if (!this.readyWaiter) return;
		clearTimeout(this.readyWaiter.timer);
		const { reject } = this.readyWaiter;
		this.readyWaiter = undefined;
		reject(error);
	}

	private resolveWaiters() {
		for (const waiter of this.waiters) {
			if (!waiter.predicate()) continue;
			clearTimeout(waiter.timer);
			this.waiters.delete(waiter);
			waiter.resolve();
		}
	}

	private rejectWaiters(error: Error) {
		for (const waiter of this.waiters) {
			clearTimeout(waiter.timer);
			waiter.reject(error);
		}
		this.waiters.clear();
	}

	private record(value: string) {
		if (this.diagnostics.length < MAX_DIAGNOSTICS) this.diagnostics.push(value);
	}

	private sendCommand(command: 'RESET' | 'ADVERTISE' | 'DISCONNECT') {
		this.report(`ESP > ${command}`);
		return this.fixture.command(command);
	}

	private setLed(rgb: string, state: string) {
		this.report(`ESP > LED ${rgb} (${state})`);
		return this.fixture.led(rgb);
	}

	private reportResponse(
		kind: RecordedRequest['kind'],
		phase: RecordedRequest['telemetryPhase'],
		bytes: number,
		fragments: number
	) {
		if (kind === 'telemetry') {
			this.telemetryResponses += 1;
			const changed = phase !== this.telemetryPhase;
			this.telemetryPhase = phase;
			if (!changed && this.telemetryResponses % 10 !== 0) return;
			this.report(
				`ESP ↔ telemetry:${phase ?? 'unknown'} response ${this.telemetryResponses} (${bytes} bytes, ${fragments} fragment${fragments === 1 ? '' : 's'})`
			);
			return;
		}
		this.report(`ESP < WRITE ${kind}`);
		this.report(
			`ESP > NOTIFY ${kind} (${bytes} bytes, ${fragments} fragment${fragments === 1 ? '' : 's'})`
		);
	}
}

export function chunks(bytes: Uint8Array, maximum: number) {
	if (!Number.isInteger(maximum) || maximum <= 0 || maximum > 244) {
		throw new Error('Notification fragment bound must be between 1 and 244');
	}
	const result: Uint8Array[] = [];
	for (let offset = 0; offset < bytes.length; offset += maximum) {
		result.push(bytes.slice(offset, Math.min(bytes.length, offset + maximum)));
	}
	return result;
}
