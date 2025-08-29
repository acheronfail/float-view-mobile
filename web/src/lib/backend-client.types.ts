import z from 'zod';

/*
 * WebSockets
 */

const createMessage = <T extends string, E extends string, D extends z.ZodTypeAny>(
	topic: T,
	event: E,
	data: D
) =>
	z.object({
		topic: z.literal(topic),
		event: z.literal(event),
		data
	});

/*
 * Common
 */

const ErrorData = z.object({ message: z.string() });
const EmptyData = z.object({});

/*
 * Socket messages
 */

export const SocketMessage = z.discriminatedUnion('event', [
	createMessage('socket', 'error', ErrorData),
	createMessage('socket', 'welcome', EmptyData)
]);
export type ZSocketMessage = z.infer<typeof SocketMessage>;

/*
 * Scan messages
 */

export const ScanStatus = z.union([z.literal('start'), z.literal('complete')]);
export type ZScanStatus = z.infer<typeof ScanStatus>;

export const ScanDevice = z.object({ address: z.string(), name: z.string() });
export type ZScanDevice = z.infer<typeof ScanDevice>;

export const ScanMessage = z.discriminatedUnion('event', [
	createMessage('scan', 'error', ErrorData),
	createMessage('scan', 'device', ScanDevice),
	createMessage('scan', 'status', z.object({ status: ScanStatus }))
]);
export type ZScanMessage = z.infer<typeof ScanMessage>;

/*
 * Connect messages
 */

const ConnectStatus = z.union([
	z.literal('start'),
	z.literal('connecting'),
	z.literal('connected')
]);

export const ConnectMessage = z.discriminatedUnion('event', [
	createMessage('connect', 'error', ErrorData),
	createMessage('connect', 'disconnect', EmptyData),
	createMessage('connect', 'status', z.object({ status: ConnectStatus }))
]);
export type ZConnectMessage = z.infer<typeof ConnectMessage>;

/*
 * All messages
 */

export const WsMessage = z.discriminatedUnion('topic', [
	SocketMessage,
	ScanMessage,
	ConnectMessage
]);

export type ZWsMessage = z.infer<typeof WsMessage>;
