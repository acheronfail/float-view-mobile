import z from 'zod';

/*
 * REST
 */

export const createApiResult = <T extends z.ZodTypeAny>(dataSchema: T) =>
	z.discriminatedUnion('kind', [
		z.object({
			kind: z.literal('success'),
			data: dataSchema
		}),
		z.object({
			kind: z.literal('error'),
			data: z.object({
				message: z.string()
			})
		})
	]);

export const BleScanDevice = z.object({ name: z.string(), address: z.string() });
export type ZBleScanDevice = z.infer<typeof BleScanDevice>;
export const BleScanResult = createApiResult(BleScanDevice.array());

export const BleConnectResult = createApiResult(z.object({ status: z.string() }));

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

const errorData = z.object({ message: z.string() });
const emptyData = z.object({});

/*
 * Socket messages
 */

export const SocketMessage = z.discriminatedUnion('event', [
	createMessage('socket', 'error', errorData),
	createMessage('socket', 'welcome', emptyData)
]);
export type ZSocketMessage = z.infer<typeof SocketMessage>;

/*
 * Scan messages
 */

export const ScanStatus = z.union([z.literal('start'), z.literal('complete')]);
export type ZScanStatus = z.infer<typeof ScanStatus>;

export const ScanMessage = z.discriminatedUnion('event', [
	createMessage('scan', 'error', errorData),
	createMessage('scan', 'device', z.object({ address: z.string(), name: z.string() })),
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
	createMessage('connect', 'error', errorData),
	createMessage('connect', 'disconnect', emptyData),
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
