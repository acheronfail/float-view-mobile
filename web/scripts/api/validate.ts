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
await validateExample(
	'sessions.list.request.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsListRequest'
);
await validateExample(
	'sessions.list.response.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsListResponse'
);
await validateExample(
	'sessions.query-series.request.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsQuerySeriesRequest'
);
await validateExample(
	'sessions.query-series.response.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsQuerySeriesResponse'
);
await validateExample(
	'sessions.export.request.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsExportRequest'
);
await validateExample(
	'sessions.export.response.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsExportResponse'
);
await validateExample(
	'sessions.delete.request.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsDeleteRequest'
);
await validateExample(
	'sessions.delete.response.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionsDeleteResponse'
);
await validateExample(
	'session-groups.get.request.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionGroupsGetRequest'
);
await validateExample(
	'session-groups.get.response.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionGroupsGetResponse'
);
await validateExample(
	'session-groups.unlink.request.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionGroupsUnlinkRequest'
);
await validateExample(
	'session-groups.unlink.response.json',
	'https://float-view.local/api/v1/schemas/product.schema.json',
	'SessionGroupsUnlinkResponse'
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
	`Validated ${schemas.length} JSON Schemas, 14 examples, and the AsyncAPI document structure.`
);
