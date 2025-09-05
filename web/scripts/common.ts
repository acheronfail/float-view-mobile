import { CommPacketId } from '../src/lib/vesc.commands';
import { RefloatCommPacketId } from '../src/lib/refloat.commands';

export type VariantSchema = {
	anyOf: ObjectSchema[];
};

export type ObjectSchema = {
	type: 'object';
	required: string[];
	properties: {
		kind: PropertyString | PropertyNumber;
		[key: string]: VariantSchema | Property;
	};
};

export type PropertyString = {
	type: 'string';
	const: string;
	description?: string;
};

export type PropertyNumber = {
	type: 'number';
	const: number;
	description?: string;
};

export type Property =
	| PropertyString
	| PropertyNumber
	| { type: 'boolean' }
	| ObjectSchema
	| VariantSchema;

export type Enums = Record<string, { [key: string]: number | string }>;
export function enumVariants(e: Enums[string]) {
	return Object.values(e)
		.filter((v): v is number => typeof v === 'number' && !Number.isNaN(v))
		.map((n): [string, number] => [e[n] as string, n]);
}

export function getKindEnumName(field: Property) {
	if (!('description' in field)) {
		throw new Error(`A description is required for getKindEnumName: ${JSON.stringify(field)}`);
	}

	switch (field.description) {
		case 'CommPacketId':
			return CommPacketId[field.const];
		case 'RefloatCommPacketId':
			return RefloatCommPacketId[field.const];
		default:
			throw new Error(`Unknown enum mapping: ${JSON.stringify(field)}`);
	}
}
