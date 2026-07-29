import { launchToken } from '$lib/auth.server.ts';
import { fixtureCoordinator } from '$lib/coordinator/resolver.server.ts';

let initialised: Promise<void> | undefined;

export async function load() {
	const coordinator = fixtureCoordinator();
	initialised ??= coordinator.initialise(process.env.FLOAT_VIEW_FIXTURE_PORT || undefined);
	await initialised;
	return {
		token: launchToken(),
		snapshot: coordinator.snapshot()
	};
}
