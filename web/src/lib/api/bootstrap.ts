import { connectApiClient, type ApiClient } from './client';

export type BootstrapDocument = {
	apiVersion: { major: number; minor: number };
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

export function connectBootstrap(document: BootstrapDocument): Promise<ApiClient> {
	const url = new URL(document.webSocketUrl, window.location.href);
	if (document.sessionToken) url.searchParams.set('token', document.sessionToken);
	return connectApiClient(url.toString());
}
