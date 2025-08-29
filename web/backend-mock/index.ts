import express from 'express';
import { createServer } from 'http';
import { WebSocketServer } from 'ws';
import { WsCommand, type ZWsMessage } from '../src/lib/backend-client.types.js';

const PORT = 3001;

const app = express();
const server = createServer(app);
const wss = new WebSocketServer({ server });

function createMessage<T extends ZWsMessage>(
	topic: T['topic'],
	event: T['event'],
	data: T['data']
): T {
	return {
		topic,
		event,
		data
	} as T;
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

wss.on('connection', (ws) => {
	const reply = (msg: ZWsMessage) => ws.send(JSON.stringify(msg));

	reply(createMessage('socket', 'welcome', {}));

	// Handle incoming messages
	ws.on('message', async (data) => {
		try {
			const message: WsCommand = JSON.parse(data.toString());
			console.debug({ message });
			switch (message.cmd) {
				case 'scan':
					reply(createMessage('scan', 'status', { status: 'start' }));
					await sleep(200);
					reply(createMessage('scan', 'device', { name: 'ExpressA', address: '11:22:33:44:55' }));
					await sleep(500);
					reply(createMessage('scan', 'device', { name: 'ExpressB', address: 'aa:bb:cc:dd:ee' }));
					await sleep(300);
					reply(createMessage('scan', 'status', { status: 'complete' }));

					break;
				case 'connect':
					reply(createMessage('connect', 'status', { status: 'start' }));
					await sleep(100);
					reply(createMessage('connect', 'status', { status: 'connecting' }));
					await sleep(500);
					reply(createMessage('connect', 'status', { status: 'connected' }));
					break;
				case 'disconnect':
					await sleep(100);
					reply(createMessage('connect', 'status', { status: 'disconnected' }));
					break;
				default:
					reply(createMessage('socket', 'error', { message: 'Unknown command' }));
			}
		} catch (error) {
			console.error('Error parsing WebSocket message:', error);
			reply(createMessage('socket', 'error', { message: 'Invalid message format' }));
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
