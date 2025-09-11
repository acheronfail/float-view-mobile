import { Property, VariantSchema, VescDataTypeToKotlinType } from '../common';

export function generateKotlinBackendMessages(schema: VariantSchema) {
	const dataClassDefinitions: Map<string, string[]> = new Map();
	const getKotlinTypeAndDefault = (
		name: string,
		def: Property,
		required: string[]
	): [string, string] => {
		const isOptional = !required.includes(name);
		const suffix = isOptional ? '?' : '';
		if ('anyOf' in def) {
			throw new Error(`Variants not supported here: ${JSON.stringify(def)}`);
		}

		if ('description' in def && typeof def.description === 'string') {
			const knownType = VescDataTypeToKotlinType[def.description];
			if (knownType) {
				const [typeName, defaultValue] = knownType;
				return [typeName + suffix, isOptional ? 'null' : defaultValue];
			}
		}

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

					const fields = Object.entries(def.properties ?? {}).map(([propName, propDef]) => [
						propName,
						...getKotlinTypeAndDefault(propName, propDef, def.required ?? [])
					]);

					for (const [propName, typeName] of fields) {
						lines.push(`  val ${propName}: ${typeName},`);
					}
					lines.push(') {');
					lines.push('  fun toJsonObject(): JsonObject {');
					lines.push('    return buildJsonObject {');
					for (const [propName] of Object.entries(def.properties ?? {})) {
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
				throw new Error(`Unsupported type: ${JSON.stringify(def)}`);
		}
	};

	const backendClassLines = ['class BackendMessage {', ''];
	backendClassLines.push(`  companion object {`);
	for (const variant of schema.anyOf ?? []) {
		if (variant.type !== 'object') throw new Error('Unsupported');

		const required = variant.required ?? [];
		const { kind, ...rest } = variant.properties ?? {};
		const { const: name } = kind;

		const parameters = Object.entries(rest).map(([propName, propDef]) => [
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

	return [
		'package fail.acheron.floatview',
		'',
		'import kotlinx.serialization.json.*',
		'',
		...dataClassDefinitions.values(),
		...backendClassLines
	]
		.flat()
		.join('\n');
}
