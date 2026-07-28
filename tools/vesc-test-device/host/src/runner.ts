import { toHex } from "./bytes.js";
import { ScenarioEngine } from "./engine.js";
import { establishControlSession } from "./lab.js";
import type { Scenario } from "./types.js";
import { UsbControlClient } from "./usb.js";

export class FixtureRunner {
  readonly engine: ScenarioEngine;
  private work: Promise<void> = Promise.resolve();
  private failure: Error | null = null;

  constructor(
    scenario: Scenario,
    private readonly control: UsbControlClient,
  ) {
    this.engine = new ScenarioEngine(scenario);
    control.on("event", (event) => {
      if (event.kind === "gattWrite" && event.accepted === true && typeof event.hex === "string") {
        this.work = this.work
          .then(() => this.handleGattWrite(Uint8Array.from(Buffer.from(event.hex as string, "hex"))))
          .catch((error: unknown) => {
            this.failure ??= error instanceof Error ? error : new Error(String(error));
          });
      }
    });
  }

  async arm(): Promise<void> {
    await establishControlSession(this.control);
    await this.control.command("reset");
    await this.control.command("configureGattProfile", { profile: "canonicalVescUart" });
    await this.control.command("setWritePolicy", { policy: "accept" });
    await this.control.command("startAdvertising");
  }

  async stop(): Promise<void> {
    try {
      await this.control.command("stopAdvertising");
    } finally {
      await this.control.close();
    }
  }

  async waitForIdle(): Promise<void> {
    await this.work;
    if (this.failure) throw this.failure;
  }

  private async handleGattWrite(chunk: Uint8Array): Promise<void> {
    const actions = await this.engine.receiveGattWrite(chunk);
    for (const action of actions) {
      if (action.kind === "disconnect") {
        await this.control.command("disconnect");
        this.engine.recordDisconnectSent(action.ruleId, action.phase);
      } else {
        for (const responseChunk of action.chunks) {
          await this.control.command("notify", { hex: toHex(responseChunk) });
          this.engine.recordNotificationSent(action.ruleId, responseChunk);
          if (action.interChunkDelayMs > 0) {
            await new Promise((resolve) => setTimeout(resolve, action.interChunkDelayMs));
          }
        }
      }
    }
  }
}
