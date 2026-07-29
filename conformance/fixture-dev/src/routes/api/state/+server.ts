import { fixtureCoordinator } from '$lib/coordinator/resolver.server.ts';

export function GET() {
	return Response.json(fixtureCoordinator().snapshot(), {
		headers: { 'cache-control': 'no-store' }
	});
}
