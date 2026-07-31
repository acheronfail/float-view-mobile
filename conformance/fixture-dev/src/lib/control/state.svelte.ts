import type { FixtureAction, FixtureSnapshot, SerialPortOption } from '../contract.ts';
import { shouldReplaceSnapshot, startBoundedPolling } from './availability.ts';
import { FixtureControlClient } from './client.ts';

export class ControlState {
  snapshot = $state<FixtureSnapshot>({} as FixtureSnapshot);
  ports = $state<SerialPortOption[]>([]);
  loading = $state(false);
  error = $state<string>();
  private readonly client: FixtureControlClient;
  private stopPolling?: () => void;

  constructor(initial: FixtureSnapshot, token: string) {
    this.snapshot = initial;
    this.client = new FixtureControlClient(token);
  }

  async start() {
    await this.refreshPorts();
    this.stopPolling ??= startBoundedPolling(() => void this.refresh(), 500);
  }

  stop() {
    this.stopPolling?.();
    this.stopPolling = undefined;
  }

  async refresh() {
    try {
      const snapshot = await this.client.state();
      this.replace(snapshot);
    } catch (error) {
      this.error = message(error);
    }
  }

  async refreshPorts() {
    try {
      this.ports = await this.client.ports();
    } catch (error) {
      this.error = message(error);
    }
  }

  async act(action: FixtureAction | Record<string, unknown>) {
    if (this.loading) return;
    this.loading = true;
    this.error = undefined;
    try {
      this.replace(await this.client.action(action));
      if ((action as FixtureAction).kind === 'attach' || (action as FixtureAction).kind === 'detach') {
        await this.refreshPorts();
      }
    } catch (error) {
      const snapshot = (error as Error & { snapshot?: FixtureSnapshot }).snapshot;
      if (snapshot) this.replace(snapshot);
      this.error = message(error);
    } finally {
      this.loading = false;
    }
  }

  private replace(snapshot: FixtureSnapshot) {
    if (shouldReplaceSnapshot(this.snapshot, snapshot)) this.snapshot = snapshot;
  }
}

function message(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
