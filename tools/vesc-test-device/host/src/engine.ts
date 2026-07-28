import { fromHex, toHex } from "./bytes.js";
import { loadFixture } from "./fixtures.js";
import { matchRequest } from "./protocol.js";
import type {
  EngineAction,
  OccurrenceResponse,
  ResponseSpec,
  Scenario,
  ScenarioRule,
  TranscriptEntry,
  VerificationManifest,
} from "./types.js";
import { encodeFrame, VescStreamAssembler } from "./vesc.js";

export interface EngineClock {
  monotonicMilliseconds(): number;
}

export class ScenarioEngine {
  private readonly assembler: VescStreamAssembler;
  private readonly counts: Record<string, number>;
  private readonly transcript: TranscriptEntry[] = [];
  private readonly requestTimes: Record<string, number[]> = {};
  private transcriptTruncated = false;
  private firstViolation: string | null = null;
  private nextExpectedIndex = 0;

  constructor(
    readonly scenario: Scenario,
    private readonly clock: EngineClock = {
      monotonicMilliseconds: () => performance.now(),
    },
  ) {
    this.assembler = new VescStreamAssembler(scenario.limits.maxBufferedBytes);
    this.counts = Object.fromEntries(
      scenario.rules.map((rule) => [rule.id, 0]),
    );
  }

  async receiveGattWrite(chunk: Uint8Array): Promise<EngineAction[]> {
    const actions: EngineAction[] = [];
    for (const result of this.assembler.append(chunk)) {
      if (result.kind === "fault") {
        this.violate(`stream ${result.code}: ${result.message}`);
        this.record(
          "internal",
          "streamFault",
          "fault",
          undefined,
          undefined,
          undefined,
          result.message,
        );
        continue;
      }
      const request = matchRequest(result.payload);
      if (request.disposition !== "readOnly") {
        const detail = `${request.disposition} command ${request.commandId} rejected`;
        this.violate(detail);
        this.record(
          "clientToFixture",
          request.disposition,
          "rejected",
          undefined,
          request.canId,
          toHex(result.frame),
          detail,
        );
        continue;
      }
      const rule = this.findRule(request.kind, request.canId);
      if (!rule) {
        const detail = `no rule for ${request.kind}${request.canId === undefined ? "" : ` on CAN ${request.canId}`}`;
        this.violate(detail);
        this.record(
          "clientToFixture",
          request.kind,
          "rejected",
          undefined,
          request.canId,
          toHex(result.frame),
          detail,
        );
        continue;
      }
      this.counts[rule.id] = (this.counts[rule.id] ?? 0) + 1;
      const count = this.counts[rule.id]!;
      const timestamp = this.clock.monotonicMilliseconds();
      (this.requestTimes[rule.id] ??= []).push(timestamp);
      if (count > rule.occurrences.maximum)
        this.violate(
          `rule ${rule.id} exceeded maximum occurrence ${rule.occurrences.maximum}`,
        );
      this.verifyOrder(rule.id);
      const selected = this.selectOutcome(rule, count);
      this.record(
        "clientToFixture",
        request.kind,
        "accepted",
        rule.id,
        request.canId,
        toHex(result.frame),
        undefined,
        count,
        selected?.phase,
        timestamp,
      );
      const action = await this.responseAction(rule, selected);
      if (action) actions.push(action);
    }
    return actions;
  }

  recordNotificationSent(ruleId: string, chunk: Uint8Array): void {
    this.record(
      "fixtureToClient",
      "notification",
      "sent",
      ruleId,
      undefined,
      toHex(chunk),
    );
  }

  recordDisconnectSent(ruleId: string, phase: string): void {
    this.record(
      "fixtureToClient",
      "disconnect",
      "sent",
      ruleId,
      undefined,
      undefined,
      undefined,
      undefined,
      phase,
    );
  }

  verify(): VerificationManifest {
    for (const rule of this.scenario.rules) {
      const count = this.counts[rule.id] ?? 0;
      if (count < rule.occurrences.minimum) {
        this.violate(
          `rule ${rule.id} expected at least ${rule.occurrences.minimum} occurrence(s), observed ${count}`,
        );
        break;
      }
    }
    if (
      this.nextExpectedIndex < this.scenario.expectations.orderedRuleIds.length
    ) {
      this.violate(
        `expected ordered rule ${this.scenario.expectations.orderedRuleIds[this.nextExpectedIndex]} was not observed`,
      );
    }
    const cadence = this.verifyCadence();
    return {
      schemaVersion: 1,
      scenarioId: this.scenario.id,
      passed: this.firstViolation === null,
      firstViolation: this.firstViolation,
      ruleCounts: { ...this.counts },
      transcriptEntries: this.transcript.length,
      transcriptTruncated: this.transcriptTruncated,
      ...(cadence.length === 0 ? {} : { cadence }),
    };
  }

  transcriptSnapshot(): readonly TranscriptEntry[] {
    return this.transcript;
  }

  private findRule(
    kind: string,
    canId: number | undefined,
  ): ScenarioRule | undefined {
    return this.scenario.rules.find(
      (rule) => rule.match.kind === kind && rule.match.canId === canId,
    );
  }

  private verifyOrder(ruleId: string): void {
    const expected =
      this.scenario.expectations.orderedRuleIds[this.nextExpectedIndex];
    if (expected === ruleId) {
      this.nextExpectedIndex += 1;
      return;
    }
    if (
      this.scenario.expectations.orderedRuleIds
        .slice(0, this.nextExpectedIndex)
        .includes(ruleId)
    ) {
      return;
    }
    this.violate(
      `expected ordered rule ${expected ?? "<none>"} before ${ruleId}`,
    );
  }

  private selectOutcome(
    rule: ScenarioRule,
    occurrence: number,
  ): OccurrenceResponse | undefined {
    if (!rule.responses) return undefined;
    const scheduledOccurrence = rule.repeatEveryOccurrences
      ? ((occurrence - 1) % rule.repeatEveryOccurrences) + 1
      : occurrence;
    const selected = rule.responses.find(
      (candidate) =>
        scheduledOccurrence >= candidate.fromOccurrence &&
        scheduledOccurrence <=
          (candidate.toOccurrence ?? Number.MAX_SAFE_INTEGER),
    );
    if (!selected) {
      this.violate(
        `rule ${rule.id} has no response for occurrence ${scheduledOccurrence}`,
      );
    }
    return selected;
  }

  private async responseAction(
    rule: ScenarioRule,
    selected: OccurrenceResponse | undefined,
  ): Promise<EngineAction | null> {
    const outcome = selected?.outcome ?? rule.response;
    if (!outcome) return null;
    if ("action" in outcome) {
      return { kind: "disconnect", ruleId: rule.id, phase: selected!.phase };
    }
    return this.notificationAction(rule.id, outcome, selected?.phase);
  }

  private async notificationAction(
    ruleId: string,
    response: ResponseSpec,
    phase?: string,
  ): Promise<EngineAction> {
    let frame: Uint8Array;
    let fixtureSizes: number[] | undefined;
    if ("fixture" in response) {
      const loaded = await loadFixture(response.fixture, response.case);
      frame = loaded.frame;
      fixtureSizes = loaded.chunkSizes;
    } else {
      frame = encodeFrame(fromHex(response.payloadHex));
    }
    const chunking = response.chunking;
    let sizes: number[];
    if (chunking.mode === "single") sizes = [frame.length];
    else if (chunking.mode === "fixture") {
      if (!fixtureSizes)
        throw new Error(
          `rule ${ruleId} requests fixture chunking but fixture has no chunkSizes`,
        );
      sizes = fixtureSizes;
    } else sizes = chunking.sizes;
    const chunks = splitExact(frame, sizes, ruleId);
    return {
      kind: "notify",
      ruleId,
      ...(phase === undefined ? {} : { phase }),
      chunks,
      interChunkDelayMs:
        chunking.mode === "sizes" ? (chunking.interChunkDelayMs ?? 0) : 0,
    };
  }

  private verifyCadence() {
    return (this.scenario.expectations.cadence ?? []).map((expectation) => {
      const times = this.requestTimes[expectation.ruleId] ?? [];
      const intervals: number[] = [];
      let passed = true;
      const repeatCount = expectation.repeatCount ?? 1;
      const repeatEvery = expectation.repeatEveryOccurrences ?? 0;
      for (let repetition = 0; repetition < repeatCount; repetition += 1) {
        for (
          let baseOccurrence = expectation.fromOccurrence;
          baseOccurrence <= expectation.toOccurrence;
          baseOccurrence += 1
        ) {
          const occurrence = baseOccurrence + repetition * repeatEvery;
          const current = times[occurrence - 1];
          const previous = times[occurrence - 2];
          if (current === undefined || previous === undefined) {
            passed = false;
            continue;
          }
          const interval = current - previous;
          intervals.push(interval);
          if (
            Math.abs(interval - expectation.intervalMs) >
            expectation.toleranceMs
          ) {
            passed = false;
            this.violate(
              `cadence ${expectation.ruleId} occurrence ${occurrence} expected ` +
                `${expectation.intervalMs}±${expectation.toleranceMs}ms, observed ${interval}ms`,
            );
          }
        }
      }
      return {
        ruleId: expectation.ruleId,
        phase: expectation.phase,
        fromOccurrence: expectation.fromOccurrence,
        toOccurrence: expectation.toOccurrence,
        expectedIntervalMs: expectation.intervalMs,
        toleranceMs: expectation.toleranceMs,
        minimumObservedMs: intervals.length === 0 ? 0 : Math.min(...intervals),
        maximumObservedMs: intervals.length === 0 ? 0 : Math.max(...intervals),
        averageObservedMs:
          intervals.length === 0
            ? 0
            : intervals.reduce((sum, value) => sum + value, 0) /
              intervals.length,
        passed,
      };
    });
  }

  private violate(message: string): void {
    this.firstViolation ??= message;
  }

  private record(
    direction: TranscriptEntry["direction"],
    kind: string,
    outcome: TranscriptEntry["outcome"],
    ruleId?: string,
    canId?: number,
    hex?: string,
    detail?: string,
    occurrence?: number,
    phase?: string,
    monotonicMs?: number,
  ): void {
    if (this.transcript.length >= this.scenario.limits.maxTranscriptEntries) {
      this.transcriptTruncated = true;
      return;
    }
    const base = {
      index: this.transcript.length,
      monotonicMs: monotonicMs ?? this.clock.monotonicMilliseconds(),
      direction,
      kind,
      outcome,
    };
    this.transcript.push({
      ...base,
      ...(ruleId === undefined ? {} : { ruleId }),
      ...(canId === undefined ? {} : { canId }),
      ...(hex === undefined ? {} : { hex }),
      ...(detail === undefined ? {} : { detail }),
      ...(occurrence === undefined ? {} : { occurrence }),
      ...(phase === undefined ? {} : { phase }),
    });
  }
}

function splitExact(
  frame: Uint8Array,
  sizes: number[],
  ruleId: string,
): Uint8Array[] {
  if (sizes.some((size) => !Number.isInteger(size) || size <= 0)) {
    throw new Error(`rule ${ruleId} has invalid chunk sizes`);
  }
  if (sizes.reduce((sum, size) => sum + size, 0) !== frame.length) {
    throw new Error(
      `rule ${ruleId} chunk sizes do not total frame length ${frame.length}`,
    );
  }
  let offset = 0;
  return sizes.map((size) => {
    const result = frame.slice(offset, offset + size);
    offset += size;
    return result;
  });
}
