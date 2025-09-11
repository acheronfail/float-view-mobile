import { camelCase, pascalCase } from 'change-case';
import { getKindEnumName, Property, VariantSchema, VescDataTypeToSwiftType } from '../common';

function getTypeName(fieldName: string, fieldSchema: Property, required: string[]) {
	const isOptional = !required.includes(fieldName);
	const suffix = isOptional ? '?' : '';
	if ('anyOf' in fieldSchema) {
		throw new Error(`Variants not supported here: ${JSON.stringify(fieldSchema)}`);
	}

	if ('description' in fieldSchema && typeof fieldSchema.description === 'string') {
		const knownType = VescDataTypeToSwiftType[fieldSchema.description];
		if (knownType) {
			const [typeName] = knownType;
			return typeName + suffix;
		}
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

function generateEnums(className: string, schema: VariantSchema, classes: Map<string, string[]>) {
	const lines = [`enum ${className}: Codable {`];

	const variants = schema.anyOf.map((variant) => {
		const { kind, ...rest } = variant.properties;
		const name = kind.type === 'string' ? kind.const : camelCase(getKindEnumName(kind));
		const parameters = Object.entries(rest).map(([name, def]) => {
			const typeName = 'anyOf' in def ? pascalCase(name) : getTypeName(name, def, variant.required);
			if ('anyOf' in def) {
				if (!classes.has(typeName)) {
					classes.set(typeName, generateEnums(typeName, def, classes));
				}
			}
			return { name, typeName };
		});

		return { kind, name, parameters, required: variant.required };
	});

	for (const { name, parameters } of variants) {
		if (parameters.length === 0) {
			lines.push(`  case ${name}`);
		} else {
			const parametersString = parameters
				.map(({ name, typeName }) => `${name}: ${typeName}`)
				.join(', ');
			lines.push(`  case ${name}(${parametersString})`);
		}
	}

	lines.push('');
	lines.push('  private enum CodingKeys: String, CodingKey {');
	lines.push('    case kind');
	const codingKeys = new Set(variants.map((v) => v.parameters.map((p) => p.name)).flat());
	for (const codingKey of codingKeys) {
		lines.push(`    case ${codingKey}`);
	}
	lines.push('  }');
	lines.push('');

	const kindType = variants[0].kind.type === 'string' ? 'String' : 'Int';
	lines.push('  init(from decoder: Decoder) throws {');
	lines.push('    let container = try decoder.container(keyedBy: CodingKeys.self)');
	lines.push(`    let kind = try container.decode(${kindType}.self, forKey: .kind)`);
	lines.push('');
	lines.push('    switch kind {');
	for (const { kind, name, parameters } of variants) {
		lines.push(`    case ${kind.type === 'string' ? `"${kind.const}"` : kind.const}:`);
		for (const param of parameters) {
			lines.push(
				`      let ${param.name} = try container.decode(${param.typeName}.self, forKey: .${param.name})`
			);
		}
		if (parameters.length === 0) {
			lines.push(`      self = .${name}`);
		} else {
			lines.push(
				`      self = .${name}(${parameters.map(({ name }) => `${name}: ${name}`).join(', ')})`
			);
		}
	}
	lines.push('    default:');
	lines.push('      throw DecodingError.dataCorruptedError(');
	lines.push('        forKey: .kind,');
	lines.push('        in: container,');
	lines.push('        debugDescription: "Unknown kind: \\(kind)"');
	lines.push('      )');
	lines.push('    }');
	lines.push('  }');
	lines.push('');

	lines.push('  func encode(to encoder: Encoder) throws {');
	lines.push('    var container = encoder.container(keyedBy: CodingKeys.self)');
	lines.push('    switch self {');
	for (const { name, parameters } of variants) {
		if (parameters.length === 0) {
			lines.push(`    case .${name}:`);
			lines.push(`      try container.encode("${name}", forKey: .kind)`);
		} else {
			lines.push(`    case .${name}(${parameters.map((p) => `let ${p.name}`).join(', ')}):`);
			lines.push(`      try container.encode("${name}", forKey: .kind)`);
			for (const param of parameters) {
				lines.push(`      try container.encode(${param.name}, forKey: .${param.name})`);
			}
		}
	}
	lines.push('    }');
	lines.push('  }');
	lines.push('}');
	lines.push('');

	return lines;
}

export function generateSwiftBackendCommands(schema: VariantSchema) {
	const classes = new Map<string, string[]>();
	const backendCommandLines = generateEnums('BackendCommand', schema, classes);
	return [
		'import Foundation',
		'',
		...Array.from(classes.values()).flat(),
		...backendCommandLines
	].join('\n');
}
