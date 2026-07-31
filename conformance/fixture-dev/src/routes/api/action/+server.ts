import { handleActionRequest } from '$lib/actions.server.ts';
import { fixtureCoordinator } from '$lib/coordinator/resolver.server.ts';

export function POST({ request }: { request: Request }) {
  return handleActionRequest(request, fixtureCoordinator());
}
