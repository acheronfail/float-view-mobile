import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import z from 'zod';
import { BackendMessage } from '../src/lib/backend-client.types';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Schema = any;

// -- Kotlin

function generateKotlin(banner: string, schema: Schema, outputPath: string) {
	const dataClassDefinitions: Map<string, string[]> = new Map();
	const getKotlinTypeAndDefault = (
		name: string,
		def: Schema,
		required: string[]
	): [string, string] => {
		const isOptional = !required.includes(name);
		const suffix = isOptional ? '?' : '';
		switch (def.type) {
			case 'string':
				return ['String' + suffix, isOptional ? 'null' : '""'];
			case 'number':
				return ['Double' + suffix, isOptional ? 'null' : '0.0'];
			case 'boolean':
				return ['Boolean' + suffix, isOptional ? 'null' : 'false'];
			case 'object': {
				const typeName = `${name[0].toUpperCase() + name.slice(1)}Object`;
				if (!dataClassDefinitions.has(typeName)) {
					const lines = [`data class ${typeName}(`];

					const fields = Object.entries<Schema>(def.properties ?? {}).map(([propName, propDef]) => [
						propName,
						...getKotlinTypeAndDefault(propName, propDef, def.required ?? [])
					]);

					for (const [propName, typeName] of fields) {
						lines.push(`  val ${propName}: ${typeName},`);
					}
					lines.push(') {');
					lines.push('  fun toJsonObject(): JsonObject {');
					lines.push('    return buildJsonObject {');
					for (const [propName] of Object.entries<Schema>(def.properties ?? {})) {
						lines.push(`      put("${propName}", ${propName})`);
					}
					lines.push('    }');
					lines.push('  }');
					lines.push('');
					lines.push('  companion object {');
					lines.push('    fun build(init: ' + typeName + 'Builder.() -> Unit): ' + typeName + ' {');
					lines.push('      return ' + typeName + 'Builder().apply(init).build()');
					lines.push('    }');
					lines.push('  }');
					lines.push('}');
					lines.push('');

					lines.push('class ' + typeName + 'Builder {');
					for (const [propName, typeName, defaultValue] of fields) {
						lines.push(`  var ${propName}: ${typeName} = ${defaultValue}`);
					}
					lines.push('');
					lines.push('  fun build(): ' + typeName + ' {');
					lines.push('    return ' + typeName + '(');
					for (const [propName] of fields) {
						lines.push(`      ${propName} = ${propName},`);
					}
					lines.push('    )');
					lines.push('  }');
					lines.push('}');
					lines.push('');
					dataClassDefinitions.set(typeName, lines);
				}

				return [typeName, isOptional ? 'null' : '?'];
			}
			default:
				throw new Error(`Unsupported type: ${def.type}`);
		}
	};

	const backendClassLines = ['class BackendMessage {', ''];
	backendClassLines.push(`  companion object {`);
	for (const variant of schema.anyOf ?? []) {
		if (variant.type !== 'object') throw new Error('Unsupported');

		const required = variant.required ?? [];
		const { kind, ...rest } = variant.properties ?? {};
		const { const: name } = kind;

		const parameters = Object.entries<Schema>(rest).map(([propName, propDef]) => [
			propName,
			...getKotlinTypeAndDefault(propName, propDef, required)
		]);

		const parameterString = parameters
			.map(([propName, typeName]) => `${propName}: ${typeName}`)
			.join(', ');
		backendClassLines.push(`    fun ${name}(${parameterString}): JsonObject {`);
		backendClassLines.push(`      return buildJsonObject {`);
		backendClassLines.push(`        put("kind", "${name}")`);
		for (const [propName, typeName] of parameters) {
			if (dataClassDefinitions.has(typeName)) {
				backendClassLines.push(`        put("${propName}", ${propName}.toJsonObject())`);
			} else {
				backendClassLines.push(`        put("${propName}", ${propName})`);
			}
		}
		backendClassLines.push(`      }`);
		backendClassLines.push(`    }`);
		backendClassLines.push(``);
	}
	backendClassLines.push(`  }`);
	backendClassLines.push('}');

	const source = [
		banner,
		'package fail.acheron.floatview',
		'',
		'import kotlinx.serialization.json.*',
		'',
		...dataClassDefinitions.values(),
		...backendClassLines
	]
		.flat()
		.join('\n');

	fs.writeFileSync(outputPath, source);
}

// -- Swift

function generateSwift(banner: string, schema: Schema, outputPath: string) {
	const structDefinitions: Map<string, string[]> = new Map();
	const getSwiftType = (name: string, def: Schema, required: string[]): [string, string] => {
		const isOptional = !required.includes(name);
		const suffix = isOptional ? '?' : '';
		switch (def.type) {
			case 'string':
				return ['String' + suffix, isOptional ? 'nil' : '""'];
			case 'number':
				return ['Double' + suffix, isOptional ? 'nil' : '0'];
			case 'boolean':
				return ['Bool' + suffix, isOptional ? 'nil' : 'false'];
			case 'object': {
				const typeName = `${name[0].toUpperCase() + name.slice(1)}Object`;
				if (!structDefinitions.has(typeName)) {
					const fields = Object.entries<Schema>(def.properties ?? {}).map(([propName, propDef]) => [
						propName,
						...getSwiftType(propName, propDef, def.required ?? [])
					]);

					const structLines = [`struct ${typeName}: Codable {`];
					for (const [propName, typeName] of fields) {
						structLines.push(`  let ${propName}: ${typeName}`);
					}

					structLines.push('');
					structLines.push('  func toDictionary() -> [String:Any?] {');
					structLines.push('    return [');
					for (const [propName] of Object.entries<Schema>(def.properties ?? {})) {
						structLines.push(`      "${propName}": ${propName},`);
					}
					structLines.push('    ]');
					structLines.push('}');
					structLines.push('');
					structLines.push(
						'  static func build(_ configure: (inout Builder) -> Void) -> ' + typeName + ' {'
					);
					structLines.push('    var builder = Builder()');
					structLines.push('    configure(&builder)');
					structLines.push('    return builder.build()');
					structLines.push('  }');
					structLines.push('');
					structLines.push('  struct Builder {');
					for (const [propName, typeName, defaultValue] of fields) {
						structLines.push(`    var ${propName}: ${typeName} = ${defaultValue}`);
					}
					structLines.push('');
					structLines.push('    func build() -> ' + typeName + ' {');
					structLines.push('      return ' + typeName + '(');
					for (const [propName] of fields) {
						structLines.push(`        ${propName}: ${propName},`);
					}
					structLines.push('      )');
					structLines.push('    }');

					structLines.push('  }');
					structLines.push('}');
					structLines.push('');
					structDefinitions.set(typeName, structLines);
				}
				return [typeName, isOptional ? 'nil' : '?'];
			}
			default:
				throw new Error(`Unsupported type: ${def.type}`);
		}
	};

	const backendClassLines = ['class BackendMessage {', ''];
	for (const variant of schema.anyOf ?? []) {
		if (variant.type !== 'object') throw new Error('Unsupported');

		const required = variant.required ?? [];
		const { kind, ...rest } = variant.properties ?? {};
		const { const: name } = kind;

		const parameters = Object.entries<Schema>(rest).map(([propName, propDef]) => [
			propName,
			...getSwiftType(propName, propDef, required)
		]);
		const parameterString = parameters
			.map(([propName, typeName]) => `${propName}: ${typeName}`)
			.join(', ');

		backendClassLines.push(`  static func ${name}(${parameterString}) -> [String:Any] {`);
		backendClassLines.push(`    return [`);
		backendClassLines.push(`      "kind": "${name}",`);
		for (const [propName, typeName] of parameters) {
			if (structDefinitions.has(typeName)) {
				backendClassLines.push(`      "${propName}": ${propName}.toDictionary(),`);
			} else {
				backendClassLines.push(`      "${propName}": ${propName},`);
			}
		}
		backendClassLines.push(`    ]`);
		backendClassLines.push(`  }`);
		backendClassLines.push(``);
	}
	backendClassLines.push('}');

	const source = [banner, ...structDefinitions.values(), ...backendClassLines].flat().join('\n');

	fs.writeFileSync(outputPath, source);
}

// generate

const schema: Schema = z.toJSONSchema(BackendMessage);
const banner = `\
// This file is generated by scripts/generate.ts
// Re-generate with \`just generate\`
`;

const thisFile = fileURLToPath(import.meta.url);
const thisDir = fs.realpathSync(path.dirname(thisFile));
const projectRoot = fs.realpathSync(path.join(thisDir, '..', '..'));
process.chdir(projectRoot);

generateKotlin(
	banner,
	schema,
	'android/app/src/main/java/fail/acheron/floatview/BackendMessage.kt'
);
generateSwift(banner, schema, 'ios/float-view/BackendMessage.swift');
