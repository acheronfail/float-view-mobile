import express from 'express';
import { createServer } from 'http';
import { WebSocketServer } from 'ws';
import { WsCommand, ZBackendMessage, ZDataLogResponse } from '../src/lib/backend-client.types.js';

const PORT = 3001;

const app = express();
const server = createServer(app);
const wss = new WebSocketServer({ server });

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

wss.on('connection', (ws) => {
	const reply = (msg: ZBackendMessage) => ws.send(JSON.stringify(msg));

	reply({ kind: 'socketWelcome' });

	let logInterval: NodeJS.Timeout | null = null;
	ws.on('message', async (data) => {
		try {
			const message: WsCommand = JSON.parse(data.toString());
			console.debug({ message });
			switch (message.cmd) {
				case 'scan':
					reply({ kind: 'scanStart' });
					await sleep(200);
					reply({ kind: 'scanDevice', name: 'ExpressA', address: '11:22:33:44:55' });
					await sleep(500);
					reply({ kind: 'scanDevice', name: 'ExpressB', address: 'aa:bb:cc:dd:ee' });
					await sleep(300);
					reply({ kind: 'scanComplete' });
					break;
				case 'connect':
					{
						const address = 'address' in message.data ? message.data['address'] : null;
						if (!address) return console.error('No address in message', message);

						reply({ kind: 'connectStart', address });
						await sleep(500);
						reply({ kind: 'connectComplete', address });
						await sleep(500);
						reply({ kind: 'connectReady', address });
					}
					break;
				case 'disconnect':
					{
						const address = 'address' in message.data ? message.data['address'] : null;
						if (!address) return console.error('No address in message', message);

						await sleep(100);
						reply({ kind: 'disconnectComplete', address });
						if (logInterval) {
							clearInterval(logInterval);
							logInterval = null;
						}
					}
					break;
				case 'command':
					{
						const command = 'command' in message.data ? message.data['command'] : null;
						const payload = 'payload' in message.data ? message.data['payload'] : null;
						const address = 'address' in message.data ? message.data['address'] : null;
						await sleep(100);
						switch (command) {
							case 0:
								reply({
									kind: 'commandFirmware',
									address: address!,
									firmware: {
										major: 6,
										minor: 6,
										betaVersion: 42,
										hwType: 0,
										isPaired: false,
										name: 'TestDevice',
										uuid: '0000-0000-0000-0000-000000000000'
									}
								});
								break;
							default:
								console.error('Unknown command:', { command, payload });
								break;
						}
					}
					break;
				case 'data': {
					const address = 'address' in message.data ? message.data['address'] : null;
					if ('log' in message.data && message.data.log) {
						const getMockData = mockDataCreator();
						logInterval = setInterval(
							() => reply({ kind: 'logData', address: address!, data: getMockData() }),
							message.data.intervalMs
						);
					} else if (logInterval) {
						clearInterval(logInterval);
					}
					break;
				}
				default:
					reply({ kind: 'socketError', message: 'Unknown command' });
			}
		} catch (error) {
			console.error('Error parsing WebSocket message:', error);
			reply({ kind: 'socketError', message: 'Invalid message format' });
		}
	});

	ws.on('close', () => {
		console.log('WebSocket connection closed');
	});

	ws.on('error', (error) => {
		console.error('WebSocket error:', error);
	});
});

server.listen(PORT, () => {
	console.log(`Mock server started on ${PORT}`);
});

function mockDataCreator(): () => ZDataLogResponse {
	const randBw = (min: number, max: number) => Math.random() * (max - min) + min;
	const d = {
		adc1: randBw(2, 3.2),
		adc2: randBw(2, 3.2),
		ah: 0,
		ahCharged: 0,
		altitude: randBw(100, 120),
		bmsFault: 0,
		bmsTemp: 0,
		bmsTempBattery: 0,
		currentBattery: randBw(-10, 15),
		currentBooster: 0,
		currentFieldWeakening: 0,
		currentMotor: randBw(-40, 50),
		distance: 42,
		duty: randBw(0, 95),
		erpm: randBw(0, 5000),
		gpsAccuracy: 0,
		gpsLatitude: 0,
		gpsLongitude: 0,
		motorFault: 0,
		pitch: randBw(-10, 10),
		requestedAmps: 0,
		roll: randBw(-10, 10),
		setpoint: randBw(-10, 10),
		setpointAtr: 0,
		setpointBreakTilt: 0,
		setpointCarve: 0,
		setpointRemote: 0,
		setpointTorqueTilt: 0,
		speed: randBw(0, 45),
		state: 'RIDING',
		stateRaw: 0,
		tempBattery: 0,
		tempMosfet: 0,
		tempMotor: 0,
		time: 0,
		truePitch: randBw(-10, 10),
		voltage: randBw(60, 84),
		wh: 0,
		whCharged: 0
	};

	function smooth(current: number, min: number, max: number, maxChange: number = 0.1): number {
		const change = (Math.random() - 0.5) * 2 * maxChange * (max - min);
		return Math.max(min, Math.min(max, current + change));
	}

	return () => {
		d.adc1 = smooth(d.adc1, 2, 3.2, 0.05);
		d.adc2 = smooth(d.adc2, 2, 3.2, 0.05);
		d.altitude = smooth(d.altitude, 100, 120, 0.02);
		d.duty = smooth(d.duty, 0, 95, 0.1);
		d.erpm = smooth(d.erpm ?? 0, 0, 5000, 0.1);
		d.pitch = smooth(d.pitch, -10, 10, 0.2);
		d.roll = smooth(d.roll, -10, 10, 0.2);
		d.setpoint = smooth(d.setpoint ?? 0, -10, 10, 0.15);
		d.speed = smooth(d.speed, 0, 45, 0.08);
		d.truePitch = smooth(d.truePitch, -10, 10, 0.2);
		d.voltage = smooth(d.voltage, 60, 84, 0.03);

		return d;
	};
}
