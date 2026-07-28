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
  return value as Scenario;
}
