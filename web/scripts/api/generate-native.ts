import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { camelCase, constantCase } from 'change-case';

type Schema = {
	$ref?: string;
	const?: string | number | boolean | null;
	description?: string;
	enum?: Array<string | number | boolean | null>;
	oneOf?: Schema[];
	type?: string | string[];
	items?: Schema;
	properties?: Record<string, Schema>;
	required?: string[];
	additionalProperties?: boolean | Schema;
	$defs?: Record<string, Schema>;
};

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDir, '../../..');
const schemaDir = path.join(projectRoot, 'api/schemas');
const checkOnly = process.argv.includes('--check');

function refName(ref: string): string {
	const name = ref.split('/').at(-1);
	if (!name) throw new Error(`Invalid schema reference: ${ref}`);
	return name;
}

function primitiveType(schema: Schema, language: 'swift' | 'kotlin'): string | undefined {
	const type = Array.isArray(schema.type) ? undefined : schema.type;
	switch (type) {
		case 'string':
			return language === 'swift' ? 'String' : 'String';
		case 'number':
			return language === 'swift' ? 'Double' : 'Double';
		case 'integer':
			return language === 'swift' ? 'Int' : 'Int';
		case 'boolean':
			return language === 'swift' ? 'Bool' : 'Boolean';
		case 'null':
			return language === 'swift' ? 'JSONValue' : 'JsonElement';
		default:
			return undefined;
	}
}

function swiftType(schema: Schema): string {
	if (schema.$ref) return refName(schema.$ref);
	if (schema.const !== undefined) {
		if (typeof schema.const === 'string') return 'String';
		if (typeof schema.const === 'number') return Number.isInteger(schema.const) ? 'Int' : 'Double';
		if (typeof schema.const === 'boolean') return 'Bool';
		return 'JSONValue';
	}
	if (schema.enum) {
		const types = new Set(schema.enum.map((value) => typeof value));
		if (types.size === 1 && types.has('string')) return 'String';
		if (types.size === 1 && types.has('number')) return 'Int';
		return 'JSONValue';
	}
	if (schema.oneOf) {
		const types = new Set(schema.oneOf.map(swiftType));
		return types.size === 1 ? [...types][0] : 'JSONValue';
	}
	if (Array.isArray(schema.type)) {
		const types = new Set(schema.type.map((type) => swiftType({ type })));
		return types.size === 1 ? [...types][0] : 'JSONValue';
	}
	const primitive = primitiveType(schema, 'swift');
	if (primitive) return primitive;
	if (schema.type === 'array') return `[${swiftType(schema.items ?? {})}]`;
	if (schema.type === 'object') return '[String: JSONValue]';
	return 'JSONValue';
}

function kotlinType(schema: Schema): string {
	if (schema.$ref) return refName(schema.$ref);
	if (schema.const !== undefined) {
		if (typeof schema.const === 'string') return 'String';
		if (typeof schema.const === 'number') return Number.isInteger(schema.const) ? 'Int' : 'Double';
		if (typeof schema.const === 'boolean') return 'Boolean';
		return 'JsonElement';
	}
	if (schema.enum) {
		const types = new Set(schema.enum.map((value) => typeof value));
		if (types.size === 1 && types.has('string')) return 'String';
		if (types.size === 1 && types.has('number')) return 'Int';
		return 'JsonElement';
	}
	if (schema.oneOf) {
		const types = new Set(schema.oneOf.map(kotlinType));
		return types.size === 1 ? [...types][0] : 'JsonElement';
	}
	if (Array.isArray(schema.type)) {
		const types = new Set(schema.type.map((type) => kotlinType({ type })));
		return types.size === 1 ? [...types][0] : 'JsonElement';
	}
	const primitive = primitiveType(schema, 'kotlin');
	if (primitive) return primitive;
	if (schema.type === 'array') return `List<${kotlinType(schema.items ?? {})}>`;
	if (schema.type === 'object') return 'Map<String, JsonElement>';
	return 'JsonElement';
}

function swiftLiteral(value: string | number | boolean | null): string {
	if (typeof value === 'string') return JSON.stringify(value);
	if (value === null) return '.null';
	return String(value);
}

function kotlinLiteral(value: string | number | boolean | null): string {
	if (typeof value === 'string') return JSON.stringify(value);
	if (value === null) return 'JsonNull';
	return String(value);
}

const swiftKeywords = new Set([
	'associatedtype',
	'class',
	'deinit',
	'enum',
	'extension',
	'fileprivate',
	'func',
	'import',
	'init',
	'inout',
	'internal',
	'let',
	'open',
	'operator',
	'private',
	'protocol',
	'public',
	'rethrows',
	'static',
	'struct',
	'subscript',
	'typealias',
	'var'
]);

function swiftEnumCase(value: string): string {
	const name = camelCase(value).replace(/^[0-9]/, 'value$&') || 'value';
	return swiftKeywords.has(name) ? `\`${name}\`` : name;
}

function swiftDeclaration(name: string, schema: Schema): string {
	if (schema.enum?.every((value) => typeof value === 'string')) {
		const cases = schema.enum
			.map((value) => `  case ${swiftEnumCase(value as string)} = ${JSON.stringify(value)}`)
			.join('\n');
		return `public enum ${name}: String, Codable, Sendable, Equatable {\n${cases}\n}`;
	}
	if (schema.type !== 'object' || !schema.properties) {
		return `public typealias ${name} = ${swiftType(schema)}`;
	}

	const required = new Set(schema.required ?? []);
	const fields = Object.entries(schema.properties);
	const properties = fields
		.map(([field, definition]) => {
			const suffix = required.has(field) ? '' : '?';
			const comment = definition.description ? `  /// ${definition.description}\n` : '';
			return `${comment}  public let ${field}: ${swiftType(definition)}${suffix}`;
		})
		.join('\n');
	const parameters = fields
		.map(([field, definition]) => {
			const optional = !required.has(field);
			const defaultValue =
				definition.const !== undefined
					? ` = ${swiftLiteral(definition.const)}`
					: optional
						? ' = nil'
						: '';
			return `    ${field}: ${swiftType(definition)}${optional ? '?' : ''}${defaultValue}`;
		})
		.join(',\n');
	const assignments = fields.map(([field]) => `    self.${field} = ${field}`).join('\n');
	return `public struct ${name}: Codable, Sendable, Equatable {
${properties}

  public init(
${parameters}
  ) {
${assignments}
  }
}`;
}

function kotlinDeclaration(name: string, schema: Schema): string {
	if (schema.enum?.every((value) => typeof value === 'string')) {
		const cases = schema.enum
			.map(
				(value) =>
					`  @SerialName(${JSON.stringify(value)}) ${constantCase(value as string).replace(/^[0-9]/, 'VALUE_$&')}`
			)
			.join(',\n');
		return `@Serializable
enum class ${name} {
${cases}
}`;
	}
	if (schema.type !== 'object' || !schema.properties) {
		return `typealias ${name} = ${kotlinType(schema)}`;
	}

	const required = new Set(schema.required ?? []);
	const fields = Object.entries(schema.properties)
		.map(([field, definition]) => {
			const optional = !required.has(field);
			const defaultValue =
				definition.const !== undefined
					? ` = ${kotlinLiteral(definition.const)}`
					: optional
						? ' = null'
						: '';
			return `  val ${field}: ${kotlinType(definition)}${optional ? '?' : ''}${defaultValue}`;
		})
		.join(',\n');
	return `@Serializable
data class ${name}(
${fields}
)`;
}

async function writeOrCheck(filePath: string, content: string): Promise<void> {
	if (!content.endsWith('\n')) content += '\n';
	if (checkOnly) {
		let current: string;
		try {
			current = await fs.readFile(filePath, 'utf8');
		} catch {
			throw new Error(`Generated API file is missing: ${path.relative(projectRoot, filePath)}`);
		}
		if (current !== content) {
			throw new Error(
				`Generated API file is stale: ${path.relative(projectRoot, filePath)}. Run npm run api:generate.`
			);
		}
		return;
	}
	await fs.mkdir(path.dirname(filePath), { recursive: true });
	await fs.writeFile(filePath, content);
}

const schemas = await Promise.all(
	(await fs.readdir(schemaDir))
		.filter((name) => name.endsWith('.schema.json'))
		.sort()
		.map(
			async (name) => JSON.parse(await fs.readFile(path.join(schemaDir, name), 'utf8')) as Schema
		)
);
const definitions = new Map<string, Schema>();
for (const schema of schemas) {
	for (const [name, definition] of Object.entries(schema.$defs ?? {})) {
		if (definitions.has(name)) throw new Error(`Duplicate API schema definition: ${name}`);
		definitions.set(name, definition);
	}
}

const swiftJsonValue = `public enum JSONValue: Codable, Sendable, Equatable {
  case string(String)
  case integer(Int)
  case number(Double)
  case boolean(Bool)
  case object([String: JSONValue])
  case array([JSONValue])
  case null

  public init(from decoder: Decoder) throws {
    let container = try decoder.singleValueContainer()
    if container.decodeNil() { self = .null }
    else if let value = try? container.decode(Bool.self) { self = .boolean(value) }
    else if let value = try? container.decode(Int.self) { self = .integer(value) }
    else if let value = try? container.decode(Double.self) { self = .number(value) }
    else if let value = try? container.decode(String.self) { self = .string(value) }
    else if let value = try? container.decode([JSONValue].self) { self = .array(value) }
    else if let value = try? container.decode([String: JSONValue].self) { self = .object(value) }
    else {
      throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unsupported JSON value")
    }
  }

  public func encode(to encoder: Encoder) throws {
    var container = encoder.singleValueContainer()
    switch self {
    case .string(let value): try container.encode(value)
    case .integer(let value): try container.encode(value)
    case .number(let value): try container.encode(value)
    case .boolean(let value): try container.encode(value)
    case .object(let value): try container.encode(value)
    case .array(let value): try container.encode(value)
    case .null: try container.encodeNil()
    }
  }
}`;

const swift = `// Generated from api/schemas. Do not edit directly.
import Foundation

${swiftJsonValue}

${Array.from(definitions, ([name, schema]) => swiftDeclaration(name, schema)).join('\n\n')}
`;
const kotlin = `// Generated from api/schemas. Do not edit directly.
@file:Suppress("unused")

package fail.acheron.floatview.api.v1

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull

${Array.from(definitions, ([name, schema]) => kotlinDeclaration(name, schema)).join('\n\n')}
`;

await writeOrCheck(path.join(projectRoot, 'api/generated/swift/FloatViewAPI.swift'), swift);
await writeOrCheck(path.join(projectRoot, 'api/generated/kotlin/FloatViewAPI.kt'), kotlin);
console.log(
	checkOnly ? 'Generated native API models are current.' : 'Generated native API models.'
);
