import { SerialPort } from 'serialport';

export type FixtureEvent =
	| { kind: 'READY' | 'CONNECTED' | 'DISCONNECTED' }
	| { kind: 'WRITE'; bytes: Uint8Array }
	| { kind: 'ERROR'; code: string };

export class FixtureClient {
	private buffer = '';
	private readonly events: FixtureEvent[] = [];
	private waiter?: { resolve: (event: FixtureEvent) => void; timer: NodeJS.Timeout };

	constructor(private readonly port: SerialPort) {
		port.setEncoding('utf8');
		port.on('data', (chunk: string) => {
			this.buffer += chunk;
			if (this.buffer.length > 4_096) {
				this.buffer = '';
				throw new Error('Fixture serial buffer exceeded bound');
			}
			let newline: number;
			while ((newline = this.buffer.indexOf('\n')) >= 0) {
				const line = this.buffer.slice(0, newline).trim();
				this.buffer = this.buffer.slice(newline + 1);
				this.deliver(parseEvent(line));
			}
		});
	}

	static async open(path: string) {
		const port = new SerialPort({ path, baudRate: 115_200, autoOpen: false });
		await new Promise<void>((resolve, reject) =>
			port.open((error) => (error ? reject(error) : resolve()))
		);
		return new FixtureClient(port);
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
		const queued = this.events.shift();
		if (queued) return Promise.resolve(queued);
		if (this.waiter) return Promise.reject(new Error('Only one fixture wait may be pending'));
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				this.waiter = undefined;
				reject(new Error('Timed out waiting for fixture'));
			}, timeoutMs);
			this.waiter = { resolve, timer };
		});
	}

	async close() {
		await new Promise<void>((resolve, reject) =>
			this.port.close((error) => (error ? reject(error) : resolve()))
		);
	}

	private write(line: string) {
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
	initializing: '0000FF',
	advertising: '00FFFF',
	running: 'FFFF00',
	passed: '00FF00',
	failed: 'FF0000',
	off: '000000'
} as const;
