import { fixtureCoordinator } from '$lib/coordinator/resolver.server.ts';

export async function GET() {
  return Response.json(await fixtureCoordinator().ports(), {
    headers: { 'cache-control': 'no-store' },
  });
}
