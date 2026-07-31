import { FixtureClient } from '../../../../fixture/serial.ts';
import { FixtureCoordinator } from './coordinator.server.ts';
import { listFixturePorts } from './ports.server.ts';

const coordinatorKey = Symbol.for('float-view.fixture-dev.coordinator');
const cleanupKey = Symbol.for('float-view.fixture-dev.cleanup');

type FixtureGlobals = typeof globalThis & {
  [coordinatorKey]?: FixtureCoordinator;
  [cleanupKey]?: boolean;
};

export function fixtureCoordinator(): FixtureCoordinator {
  const globals = globalThis as FixtureGlobals;
  globals[coordinatorKey] ??= new FixtureCoordinator({
    open: FixtureClient.open,
    listPorts: listFixturePorts,
  });
  if (!globals[cleanupKey]) {
    globals[cleanupKey] = true;
    for (const signal of ['SIGINT', 'SIGTERM'] as const) {
      process.once(signal, () => {
        const deadline = setTimeout(() => process.exit(1), 2_000);
        void globals[coordinatorKey]?.cleanup().finally(() => {
          clearTimeout(deadline);
          process.exit(0);
        });
      });
    }
  }
  return globals[coordinatorKey];
}
