import express from 'express';
import { createServer } from 'http';
import { WebSocketServer } from 'ws';
import { ZBackendCommand, ZBackendMessage } from '../src/lib/backend-client.types.js';
import { CommPacketId, ZVGetValues } from '../src/lib/vesc.commands.js';
import { ZRefloatRtData } from '../src/lib/refloat.commands.js';

const PORT = 3001;

const app = express();
const server = createServer(app);
const wss = new WebSocketServer({ server });

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

wss.on('connection', (ws) => {
	const reply = (msg: ZBackendMessage) => ws.send(JSON.stringify(msg));

	reply({ kind: 'socketWelcome' });

	const getMockVGetValues = mockVGetValuesCreator();

	let logInterval: NodeJS.Timeout | null = null;
	ws.on('message', async (data) => {
		try {
			const message: ZBackendCommand = JSON.parse(data.toString());
			console.debug({ message });
			switch (message.kind) {
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
						const { address } = message;
						reply({ kind: 'connectStart', address });
						await sleep(500);
						reply({ kind: 'connectComplete', address });
						await sleep(500);
						reply({ kind: 'connectReady', address });
					}
					break;
				case 'disconnect':
					{
						const { address } = message;
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
						const { address, cmd } = message;
						await sleep(100);
						switch (cmd.kind) {
							case CommPacketId.COMM_FW_VERSION:
								reply({
									kind: 'vescFirmware',
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
							case CommPacketId.COMM_GET_VALUES:
								reply({
									kind: 'vescGetValues',
									address: address!,
									getValues: getMockVGetValues()
								});
								break;
							default:
								console.error('Unhandled command:', cmd);
								break;
						}
					}
					break;
				case 'logStart':
					{
						const { address, intervalMs } = message;
						const getMockData = mockRefloatRtDataCreator();
						logInterval = setInterval(
							() => reply({ kind: 'logData', address: address!, data: getMockData() }),
							intervalMs
						);
					}

					break;
				case 'logStop':
					if (logInterval) clearInterval(logInterval);
					break;

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

function mockRefloatRtDataCreator(): () => ZRefloatRtData {
	const randBw = (min: number, max: number) => Math.random() * (max - min) + min;
	const d: ZRefloatRtData = {
		balanceCurrent: 0,
		imuBalancePitch: randBw(-10, 10),
		imuRoll: randBw(-10, 10),
		satState: 0,
		state: 0,
		footpadState: 0,
		beepReason: 0,
		adc1: randBw(2, 3.2),
		adc2: randBw(2, 3.2),
		setpoint: randBw(-10, 10),
		setpointAtr: 0,
		setpointBrakeTilt: 0,
		setpointTorqueTilt: 0,
		setpointTurnTilt: 0,
		setpointRemote: 0,
		imuPitch: randBw(-10, 10),
		motorFilterCurrent: 0,
		atrAccelerationDiff: 0,
		boosterCurrent: 0,
		motorDirCurrent: 0,
		remoteInput: 0
	};

	function smooth(current: number, min: number, max: number, maxChange: number = 0.1): number {
		const change = (Math.random() - 0.5) * 2 * maxChange * (max - min);
		return Math.max(min, Math.min(max, current + change));
	}

	return () => {
		d.adc1 = smooth(d.adc1, 2, 3.2, 0.05);
		d.adc2 = smooth(d.adc2, 2, 3.2, 0.05);
		d.imuRoll = smooth(d.imuRoll, -10, 10, 0.2);
		d.setpoint = smooth(d.setpoint ?? 0, -10, 10, 0.15);
		d.imuBalancePitch = smooth(d.imuBalancePitch, -10, 10, 0.2);
		d.imuPitch = smooth(d.imuPitch, -10, 10, 0.2);

		return d;
	};
}

function mockVGetValuesCreator(): () => ZVGetValues {
	const randBw = (min: number, max: number) => Math.random() * (max - min) + min;
	const d: ZVGetValues = {
		tempMosfet: randBw(25, 45), // MOSFET temperature in Celsius
		tempMotor: randBw(20, 40), // Motor temperature in Celsius
		currentMotor: randBw(-5, 15), // Motor current in Amps
		currentBattery: randBw(-3, 12), // Battery current in Amps
		id: randBw(-2, 8), // Direct axis current
		iq: randBw(-10, 25), // Quadrature axis current
		dutyCycle: randBw(0, 0.3), // Duty cycle (0-1)
		erpm: randBw(-3000, 8000), // Electrical RPM
		voltage: randBw(45, 58), // Battery voltage
		ampHoursConsumed: randBw(0, 5), // Amp hours consumed
		ampHoursCharged: randBw(0, 0.5), // Amp hours charged (regenerative braking)
		wattHoursConsumed: randBw(0, 250), // Watt hours consumed
		wattHoursCharged: randBw(0, 25), // Watt hours charged
		tachValue: randBw(0, 100000), // Tachometer value
		tachAbs: randBw(0, 100000), // Absolute tachometer value
		faultCode: 0 // No fault
	};

	function smooth(current: number, min: number, max: number, maxChange: number = 0.1): number {
		const change = (Math.random() - 0.5) * 2 * maxChange * (max - min);
		return Math.max(min, Math.min(max, current + change));
	}

	return () => {
		d.tempMosfet = smooth(d.tempMosfet, 25, 45, 0.05);
		d.tempMotor = smooth(d.tempMotor, 20, 40, 0.05);
		d.currentMotor = smooth(d.currentMotor, -5, 15, 0.2);
		d.currentBattery = smooth(d.currentBattery, -3, 12, 0.2);
		d.id = smooth(d.id, -2, 8, 0.15);
		d.iq = smooth(d.iq, -10, 25, 0.3);
		d.dutyCycle = smooth(d.dutyCycle, 0, 0.3, 0.1);
		d.erpm = smooth(d.erpm, -3000, 8000, 0.2);
		d.voltage = smooth(d.voltage, 45, 58, 0.02);
		d.ampHoursConsumed = smooth(d.ampHoursConsumed, 0, 5, 0.01);
		d.ampHoursCharged = smooth(d.ampHoursCharged, 0, 0.5, 0.01);
		d.wattHoursConsumed = smooth(d.wattHoursConsumed, 0, 250, 0.01);
		d.wattHoursCharged = smooth(d.wattHoursCharged, 0, 25, 0.01);
		d.tachValue += randBw(-50, 100); // Tachometer increments
		d.tachAbs += randBw(-50, 100);

		return d;
	};
}
