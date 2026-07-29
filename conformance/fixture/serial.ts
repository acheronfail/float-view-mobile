import { SerialPort } from 'serialport';

export type FixtureEvent =
	| { kind: 'READY' | 'CONNECTED' | 'DISCONNECTED' }
	| { kind: 'WRITE'; bytes: Uint8Array }
	| { kind: 'ERROR'; code: string };

export interface FixtureControl {
	command(command: 'RESET' | 'ADVERTISE' | 'DISCONNECT'): Promise<void>;
	notify(bytes: Uint8Array): Promise<void>;
	led(rgb: string): Promise<void>;
	next(timeoutMs?: number): Promise<FixtureEvent>;
	close(): Promise<void>;
}

export class FixtureClient implements FixtureControl {
	private readonly port: SerialPort;
	private buffer = '';
	private discardFirstLine = true;
	private readonly events: FixtureEvent[] = [];
	private failure?: Error;
	private waiter?: {
		resolve: (event: FixtureEvent) => void;
		reject: (error: Error) => void;
		timer: NodeJS.Timeout;
	};

	constructor(port: SerialPort) {
		this.port = port;
		port.setEncoding('utf8');
		port.on('error', (error) => this.fail(error));
		port.on('data', (chunk: string) => {
			if (this.failure) return;
			this.buffer += chunk;
			if (this.buffer.length > 4_096) {
				this.buffer = '';
				this.fail(new Error('Fixture serial buffer exceeded bound'));
				return;
			}
			let newline: number;
			while ((newline = this.buffer.indexOf('\n')) >= 0) {
				const line = this.buffer.slice(0, newline).trim();
				this.buffer = this.buffer.slice(newline + 1);
				if (this.discardFirstLine) {
					// Native USB CDC can open after the device has already emitted
					// part of a line. RESET is retried, so discarding exactly this
					// unsynchronized line cannot lose the handshake.
					this.discardFirstLine = false;
					continue;
				}
				try {
					this.deliver(parseEvent(line));
				} catch (error) {
					this.fail(error instanceof Error ? error : new Error(String(error)));
				}
			}
		});
	}

	static async open(path: string) {
		const port = new SerialPort({ path, baudRate: 115_200, autoOpen: false });
		const client = new FixtureClient(port);
		await new Promise<void>((resolve, reject) =>
			port.open((error) => (error ? reject(error) : resolve()))
		);
		return client;
	}

	command(command: 'RESET' | 'ADVERTISE' | 'DISCONNECT') {
		return this.write(command);
	}

	notify(bytes: Uint8Array) {
		if (!bytes.length || bytes.length > 244) throw new Error('Notification exceeds fixture bound');
		return this.write(`NOTIFY ${Buffer.from(bytes).toString('hex').toUpperCase()}`);
	}

	led(rgb: string) {
		if (!/^[0-9A-F]{6}$/i.test(rgb)) throw new Error('LED color must be RRGGBB');
		return this.write(`LED ${rgb.toUpperCase()}`);
	}

	next(timeoutMs = 5_000): Promise<FixtureEvent> {
		if (this.failure) return Promise.reject(this.failure);
		const queued = this.events.shift();
		if (queued) return Promise.resolve(queued);
		if (this.waiter) return Promise.reject(new Error('Only one fixture wait may be pending'));
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				this.waiter = undefined;
				reject(new Error('Timed out waiting for fixture'));
			}, timeoutMs);
			this.waiter = { resolve, reject, timer };
		});
	}

	async close() {
		if (this.waiter) {
			clearTimeout(this.waiter.timer);
			this.waiter.reject(new Error('Fixture serial client closed'));
			this.waiter = undefined;
		}
		if (!this.port.isOpen) return;
		await new Promise<void>((resolve, reject) =>
			this.port.close((error) => (error ? reject(error) : resolve()))
		);
	}

	private write(line: string) {
		if (this.failure) return Promise.reject(this.failure);
		if (line.length > 512) return Promise.reject(new Error('Fixture command exceeds bound'));
		return new Promise<void>((resolve, reject) => {
			this.port.write(`${line}\n`, (error) => {
				if (error) reject(error);
				else this.port.drain((drainError) => (drainError ? reject(drainError) : resolve()));
			});
		});
	}

	private deliver(event: FixtureEvent) {
		if (!this.waiter) {
			if (this.events.length >= 100) throw new Error('Fixture event queue exceeded bound');
			this.events.push(event);
			return;
		}
		clearTimeout(this.waiter.timer);
		const { resolve } = this.waiter;
		this.waiter = undefined;
		resolve(event);
	}

	private fail(error: Error) {
		this.failure ??= error;
		if (!this.waiter) return;
		clearTimeout(this.waiter.timer);
		this.waiter.reject(this.failure);
		this.waiter = undefined;
	}
}

export function parseEvent(line: string): FixtureEvent {
	if (line === 'READY' || line === 'CONNECTED' || line === 'DISCONNECTED') return { kind: line };
	const write = /^WRITE ([0-9A-F]+)$/i.exec(line);
	if (write?.[1] && write[1].length % 2 === 0) {
		return { kind: 'WRITE', bytes: Uint8Array.from(Buffer.from(write[1], 'hex')) };
	}
	const error = /^ERROR ([A-Z0-9_]+)$/.exec(line);
	if (error?.[1]) return { kind: 'ERROR', code: error[1] };
	throw new Error(`Invalid fixture event: ${line}`);
}

export const led = {
	initializing: '000088',
	advertising: '008888',
	running: '880088',
	passed: '008800',
	failed: '880000',
	off: '000000'
} as const;
