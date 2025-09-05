import { Schema } from "../common";

export function generateSwiftBackendMessages(schema: Schema) {
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

	return [...structDefinitions.values(), ...backendClassLines].flat().join('\n');
}