import { pascalCase } from 'change-case';
import { getKindEnumName, Property, VariantSchema } from '../common';

function getTypeName(fieldName: string, fieldSchema: Property, required: string[]) {
	const isOptional = !required.includes(fieldName);
	const suffix = isOptional ? '?' : '';
	if ('anyOf' in fieldSchema) {
		throw new Error('Variants are not supportd here');
	}

	switch (fieldSchema.type) {
		case 'string':
			return 'String' + suffix;
		case 'number':
			return 'Double' + suffix;
		default:
			throw new Error(`Unsupported type: ${JSON.stringify({ fieldName, fieldSchema }, null, 2)}`);
	}
}

function generateSealedClass(
	className: string,
	schema: VariantSchema,
	classes: Map<string, string[]>
) {
	const lines = ['@Serializable', `sealed class ${className} {`];
	const variants = schema.anyOf.map((variant) => {
		const { kind, ...rest } = variant.properties;
		const name = pascalCase(kind.type === 'string' ? kind.const : getKindEnumName(kind));
		const parameters = Object.entries(rest);
		return { name, parameters, kind, required: variant.required };
	});

	for (const { name, parameters, kind, required } of variants) {
		lines.push('  @Serializable');
		lines.push(`  @SerialName("${kind.const}")`);
		if (parameters.length === 0) {
			lines.push(`  class ${name} : ${className}()`);
		} else {
			const parametersString = parameters
				.map(([name, def]) => {
					const typeName = 'anyOf' in def ? pascalCase(name) : getTypeName(name, def, required);
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
	for (const { name } of variants) {
		lines.push(`          subclass(${name}::class)`);
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

	const variantEnum = variants[0].kind.description;
	lines.push(`  fun kind(): ${variantEnum ? variantEnum : 'String'} {`);
	lines.push('    return when (this) {');
	for (const { name, kind } of variants) {
		lines.push(
			`      is ${name} -> ${variantEnum ? `${variantEnum}.fromInt(${kind.const})!!` : JSON.stringify(kind.const)}`
		);
	}
	lines.push('    }');
	lines.push('  }');
	lines.push('');

	lines.push('}');
	lines.push('');

	return lines;
}

export function generateKotlinBackendCommands(schema: VariantSchema) {
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
