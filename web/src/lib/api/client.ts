import type {
	ApiMethod,
	ApiParams,
	ApiResult,
	HelloParams,
	HelloResult,
	JsonRpcId,
	RpcError,
	RpcErrorData
} from '../../../../api/generated/typescript/models.js';

type PendingRequest = {
	resolve: (value: unknown) => void;
	reject: (reason: unknown) => void;
	timeout: ReturnType<typeof setTimeout>;
};

export type NotificationHandler<T = unknown> = (params: T) => void;

export class ApiRequestError extends Error {
	constructor(
		readonly code: number,
		message: string,
		readonly data: RpcErrorData
	) {
		super(message);
		this.name = 'ApiRequestError';
	}
}

export class ApiProtocolError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'ApiProtocolError';
	}
}

export class ApiClient {
	private nextId = 1;
	private readonly pending = new Map<JsonRpcId, PendingRequest>();
	private readonly notificationHandlers = new Map<string, Set<NotificationHandler>>();
	private closed = false;

	constructor(
		private readonly socket: WebSocket,
		private readonly defaultTimeoutMs = 5_000
	) {
		socket.addEventListener('message', this.handleMessage);
		socket.addEventListener('close', this.handleClose);
	}

	request<M extends ApiMethod>(
		method: M,
		params: ApiParams<M>,
		timeoutMs = this.defaultTimeoutMs
	): Promise<ApiResult<M>> {
		if (this.closed || this.socket.readyState !== 1) {
			return Promise.reject(new ApiProtocolError('API WebSocket is not open'));
		}

		const id = this.nextId++;
		return new Promise<ApiResult<M>>((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pending.delete(id);
				reject(new ApiProtocolError(`API request timed out: ${method}`));
			}, timeoutMs);

			this.pending.set(id, {
				resolve: resolve as (value: unknown) => void,
				reject,
				timeout
			});
			try {
				this.socket.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
			} catch (error) {
				clearTimeout(timeout);
				this.pending.delete(id);
				reject(error);
			}
		});
	}

	hello(params: HelloParams, timeoutMs?: number): Promise<HelloResult> {
		return this.request('system.hello', params, timeoutMs);
	}

	onNotification<T = unknown>(method: string, handler: NotificationHandler<T>): () => void {
		let handlers = this.notificationHandlers.get(method);
		if (!handlers) {
			handlers = new Set();
			this.notificationHandlers.set(method, handlers);
		}
		handlers.add(handler as NotificationHandler);
		return () => {
			handlers.delete(handler as NotificationHandler);
			if (handlers.size === 0) this.notificationHandlers.delete(method);
		};
	}

	close(code = 1000, reason = 'Client closed'): void {
		if (this.closed) return;
		this.closed = true;
		this.detach();
		this.rejectPending(new ApiProtocolError(reason));
		this.socket.close(code, reason);
	}

	private readonly handleMessage = (event: MessageEvent<unknown>): void => {
		if (typeof event.data !== 'string') return;

		let message: unknown;
		try {
			message = JSON.parse(event.data);
		} catch {
			return;
		}
		if (!message || typeof message !== 'object') return;

		const envelope = message as Record<string, unknown>;
		if (envelope.jsonrpc !== '2.0') return;

		if (isJsonRpcId(envelope.id)) {
			const request = this.pending.get(envelope.id);
			if (!request) return;
			this.pending.delete(envelope.id);
			clearTimeout(request.timeout);

			if (isRpcError(envelope.error)) {
				request.reject(
					new ApiRequestError(envelope.error.code, envelope.error.message, envelope.error.data)
				);
			} else if ('result' in envelope) {
				request.resolve(envelope.result);
			} else {
				request.reject(new ApiProtocolError('JSON-RPC response has no result or error'));
			}
			return;
		}

		if (typeof envelope.method === 'string') {
			for (const handler of this.notificationHandlers.get(envelope.method) ?? []) {
				handler(envelope.params);
			}
		}
	};

	private readonly handleClose = (): void => {
		if (this.closed) return;
		this.closed = true;
		this.detach();
		this.rejectPending(new ApiProtocolError('API WebSocket closed'));
	};

	private detach(): void {
		this.socket.removeEventListener('message', this.handleMessage);
		this.socket.removeEventListener('close', this.handleClose);
	}

	private rejectPending(error: Error): void {
		for (const request of this.pending.values()) {
			clearTimeout(request.timeout);
			request.reject(error);
		}
		this.pending.clear();
	}
}

function isJsonRpcId(value: unknown): value is JsonRpcId {
	return typeof value === 'string' || (typeof value === 'number' && Number.isInteger(value));
}

function isRpcError(value: unknown): value is RpcError {
	if (!value || typeof value !== 'object') return false;
	const error = value as Record<string, unknown>;
	const data = error.data as Record<string, unknown> | undefined;
	return (
		typeof error.code === 'number' &&
		typeof error.message === 'string' &&
		!!data &&
		typeof data.kind === 'string' &&
		typeof data.recoverable === 'boolean'
	);
}

export function connectApiClient(
	url: string,
	timeoutMs = 5_000,
	createSocket: (url: string) => WebSocket = (socketUrl) => new WebSocket(socketUrl)
): Promise<ApiClient> {
	return new Promise((resolve, reject) => {
		const socket = createSocket(url);
		const timeout = setTimeout(() => {
			cleanup();
			socket.close();
			reject(new ApiProtocolError('Timed out opening API WebSocket'));
		}, timeoutMs);
		const cleanup = () => {
			clearTimeout(timeout);
			socket.removeEventListener('open', handleOpen);
			socket.removeEventListener('error', handleError);
		};
		const handleOpen = () => {
			cleanup();
			resolve(new ApiClient(socket, timeoutMs));
		};
		const handleError = () => {
			cleanup();
			socket.close();
			reject(new ApiProtocolError('Failed to open API WebSocket'));
		};
		socket.addEventListener('open', handleOpen);
		socket.addEventListener('error', handleError);
	});
}
