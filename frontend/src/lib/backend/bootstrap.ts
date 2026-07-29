import { connectApiClient, type ApiClient } from './client';

export type BootstrapDocument = {
	webSocketUrl: string;
	sessionToken?: string;
};

export async function loadBootstrap(fetcher: typeof fetch = fetch): Promise<BootstrapDocument> {
	const response = await fetcher('/api/bootstrap', {
		cache: 'no-store',
		headers: { Accept: 'application/json' }
	});
	if (!response.ok) throw new Error(`Bootstrap failed: HTTP ${response.status}`);
	return (await response.json()) as BootstrapDocument;
}

export function bootstrapWebSocketUrl(
	document: BootstrapDocument,
	base: string,
	development: boolean
): URL {
	const url = new URL(document.webSocketUrl, base);
	if (development) {
		const frontend = new URL(base);
		url.protocol = frontend.protocol === 'https:' ? 'wss:' : 'ws:';
		url.host = frontend.host;
	}
	if (document.sessionToken) url.searchParams.set('token', document.sessionToken);
	return url;
}

export function connectBootstrap(document: BootstrapDocument): Promise<ApiClient> {
	const url = bootstrapWebSocketUrl(document, window.location.href, import.meta.env.DEV);
	return connectApiClient(url.toString());
}
