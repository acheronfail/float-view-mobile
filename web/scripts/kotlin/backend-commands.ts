import { pascalCase } from 'change-case';
import { getKindEnumName, Property, Schema } from '../common';

function getTypeName(fieldName: string, fieldSchema: Property, required: string[]) {
	const isOptional = !required.includes(fieldName);
	const suffix = isOptional ? '?' : '';
	switch (fieldSchema.type) {
		case 'string':
			return 'String' + suffix;
		case 'number':
			return 'Double' + suffix;
		default:
			throw new Error(`Unsupported type: ${JSON.stringify({ fieldName, fieldSchema }, null, 2)}`);
	}
}

function generateSealedClass(className: string, schema: Schema, classes: Map<string, string[]>) {
	const lines = ['@Serializable', `sealed class ${className} {`];
	const variantNames: string[] = [];
	for (const variant of schema.anyOf) {
		const { kind, ...rest } = variant.properties;
		const name = pascalCase(kind.type === 'string' ? kind.const : getKindEnumName(kind));
		const parameters = Object.entries(rest);
		variantNames.push(name);

		lines.push('  @Serializable');
		lines.push(`  @SerialName("${kind.const}")`);
		if (parameters.length === 0) {
			lines.push(`  class ${name} : ${className}()`);
		} else {
			const parametersString = parameters
				.map(([name, def]) => {
					const typeName =
						'anyOf' in def ? pascalCase(name) : getTypeName(name, def, variant.required);
					if ('anyOf' in def) {
						if (!classes.has(typeName)) {
							classes.set(typeName, generateSealedClass(typeName, def, classes));
						}
					}
					return `val ${name}: ${typeName}`;
				})
				.join(', ');
			lines.push(`  data class ${name}(${parametersString}) : ${className}()`);
		}

		lines.push('');
	}

	lines.push('  companion object {');
	lines.push('    private val json = Json {');
	lines.push('      serializersModule = SerializersModule {');
	lines.push(`        polymorphic(${className}::class) {`);
	for (const variantName of variantNames) {
		lines.push(`          subclass(${variantName}::class)`);
	}
	lines.push('        }');
	lines.push('        classDiscriminator = "kind"');
	lines.push('      }');
	lines.push('    }');
	lines.push('');
	lines.push(`    fun fromJson(jsonString: String): ${className} {`);
	lines.push(`      return json.decodeFromString(jsonString)`);
	lines.push(`    }`);
	lines.push('  }');
	lines.push('');

	lines.push('}');
	lines.push('');

	return lines;
}

export function generateKotlinBackendCommands(schema: Schema) {
	const classes = new Map<string, string[]>();
	const backendCommandLines = generateSealedClass('BackendCommand', schema, classes);
	return [
		'@file:Suppress("UnsafeOptInUsageError", "Unused")',
		'',
		'import kotlinx.serialization.SerialName',
		'import kotlinx.serialization.Serializable',
		'import kotlinx.serialization.json.Json',
		'import kotlinx.serialization.modules.SerializersModule',
		'import kotlinx.serialization.modules.polymorphic',
		'import kotlinx.serialization.modules.subclass',
		'',
		...Array.from(classes.values()).flat(),
		...backendCommandLines
	].join('\n');
}
