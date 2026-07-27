import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020, { type AnySchema, type ValidateFunction } from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import { parse as parseYaml } from 'yaml';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDir, '../../..');
const apiDir = path.join(projectRoot, 'api');
const schemaDir = path.join(apiDir, 'schemas');

const schemaFiles = (await fs.readdir(schemaDir))
	.filter((name) => name.endsWith('.schema.json'))
	.sort();
const schemas = await Promise.all(
	schemaFiles.map(async (name) => JSON.parse(await fs.readFile(path.join(schemaDir, name), 'utf8')))
);

const ajv = new Ajv2020({
	allErrors: true,
	allowUnionTypes: true,
	strict: true,
	validateFormats: true
});
addFormats(ajv);
for (const schema of schemas) {
	ajv.addSchema(schema as AnySchema);
}

function validator(schemaId: string, definition: string): ValidateFunction {
	const validate = ajv.getSchema(`${schemaId}#/$defs/${definition}`);
	if (!validate) {
		throw new Error(`Schema definition not found: ${schemaId}#/$defs/${definition}`);
	}
	return validate;
}

async function validateExample(name: string, schemaId: string, definition: string): Promise<void> {
	const data = JSON.parse(await fs.readFile(path.join(apiDir, 'examples', name), 'utf8'));
	const validate = validator(schemaId, definition);
	if (!validate(data)) {
		throw new Error(`${name} is invalid:\n${ajv.errorsText(validate.errors, { separator: '\n' })}`);
	}
}

await validateExample(
	'system.hello.request.json',
	'https://float-view.local/api/v1/schemas/system.schema.json',
	'HelloRequest'
);
await validateExample(
	'system.hello.response.json',
	'https://float-view.local/api/v1/schemas/system.schema.json',
	'HelloResponse'
);

const asyncApi = parseYaml(await fs.readFile(path.join(apiDir, 'asyncapi.yaml'), 'utf8')) as {
	asyncapi?: string;
	channels?: Record<string, unknown>;
	operations?: Record<string, unknown>;
};
if (
	asyncApi.asyncapi !== '3.0.0' ||
	!asyncApi.channels?.api ||
	!asyncApi.operations?.receiveHello
) {
	throw new Error('api/asyncapi.yaml is missing the v1 channel or required operations');
}

console.log(
	`Validated ${schemas.length} JSON Schemas, 2 examples, and the AsyncAPI document structure.`
);
