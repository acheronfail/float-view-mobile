import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020Import from "ajv/dist/2020.js";
import type { ErrorObject } from "ajv";
import type { Scenario } from "./types.js";

const moduleDirectory = dirname(fileURLToPath(import.meta.url));
export const repositoryRoot = resolve(moduleDirectory, "../../../..");

export async function loadScenario(path: string): Promise<Scenario> {
  const schemaPath = resolve(repositoryRoot, "tools/vesc-test-device/schemas/scenario-v1.schema.json");
  const [schemaText, scenarioText] = await Promise.all([
    readFile(schemaPath, "utf8"),
    readFile(path, "utf8"),
  ]);
  const schema = JSON.parse(schemaText) as object;
  const value: unknown = JSON.parse(scenarioText);
  const Ajv2020 = Ajv2020Import as unknown as new (options: { allErrors: boolean; strict: boolean }) => {
    compile(schema: object): {
      (value: unknown): boolean;
      errors?: ErrorObject[] | null;
    };
  };
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  const validate = ajv.compile(schema);
  if (!validate(value)) {
    const errors = validate.errors?.map((error: ErrorObject) => `${error.instancePath || "/"} ${error.message}`).join("; ");
    throw new Error(`invalid scenario: ${errors}`);
  }
  const scenario = value as Scenario;
  validateSemantics(scenario);
  return scenario;
}

function validateSemantics(scenario: Scenario): void {
  const ruleIds = new Set<string>();
  for (const rule of scenario.rules) {
    if (ruleIds.has(rule.id)) throw new Error(`invalid scenario: duplicate rule ID ${rule.id}`);
    ruleIds.add(rule.id);
    if (rule.occurrences.minimum > rule.occurrences.maximum) {
      throw new Error(`invalid scenario: rule ${rule.id} minimum exceeds maximum`);
    }
    if (rule.responses) {
      let next = 1;
      for (const response of rule.responses) {
        if (response.fromOccurrence !== next) {
          throw new Error(`invalid scenario: rule ${rule.id} response schedule has a gap or overlap at occurrence ${next}`);
        }
        const end = response.toOccurrence ?? rule.occurrences.maximum;
        if (end < response.fromOccurrence || end > rule.occurrences.maximum) {
          throw new Error(`invalid scenario: rule ${rule.id} response schedule has an invalid range`);
        }
        next = end + 1;
      }
      if (next <= rule.occurrences.maximum) {
        throw new Error(`invalid scenario: rule ${rule.id} response schedule ends before maximum occurrence`);
      }
    }
  }
  if (!scenario.topology.canNodes.some((node) => node.canId === scenario.topology.primaryCanId)) {
    throw new Error("invalid scenario: primaryCanId is absent from canNodes");
  }
  for (const id of scenario.expectations.orderedRuleIds) {
    if (!ruleIds.has(id)) throw new Error(`invalid scenario: ordered rule ${id} does not exist`);
  }
  for (const cadence of scenario.expectations.cadence ?? []) {
    const rule = scenario.rules.find((candidate) => candidate.id === cadence.ruleId);
    if (!rule) throw new Error(`invalid scenario: cadence rule ${cadence.ruleId} does not exist`);
    if (
      cadence.fromOccurrence > cadence.toOccurrence
      || cadence.toOccurrence > rule.occurrences.maximum
    ) {
      throw new Error(`invalid scenario: cadence range for ${cadence.ruleId} is invalid`);
    }
  }
}
