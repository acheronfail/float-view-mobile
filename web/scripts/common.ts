import { CommPacketId } from '../src/lib/vesc.commands';
import { RefloatCommPacketId } from '../src/lib/refloat.commands';
import { VescDataType } from '../src/lib/vesc.types';

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

export const VescDataTypeToKotlinType: Record<VescDataType, [string, string]> = {
	[VescDataType.Double16]: ['Double', '0.0'],
	[VescDataType.Double32]: ['Double', '0.0'],
	[VescDataType.Float32]: ['Float', '0.0'],
	[VescDataType.Int8]: ['Int', '0'],
	[VescDataType.Int16]: ['Int', '0'],
	[VescDataType.Int32]: ['Int', '0'],
	[VescDataType.String]: ['String', '""'],
	[VescDataType.UInt8]: ['UInt', '0'],
	[VescDataType.UInt16]: ['UInt', '0'],
	[VescDataType.UInt32]: ['UInt', '0'],
};

export const VescDataTypeToSwiftType: Record<VescDataType, [string, string]> = {
	[VescDataType.Double16]: ['Double', '0.0'],
	[VescDataType.Double32]: ['Double', '0.0'],
	[VescDataType.Float32]: ['Float', '0.0'],
	[VescDataType.Int8]: ['Int8', '0'],
	[VescDataType.Int16]: ['Int16', '0'],
	[VescDataType.Int32]: ['Int32', '0'],
	[VescDataType.String]: ['String', '""'],
	[VescDataType.UInt8]: ['UInt8', '0'],
	[VescDataType.UInt16]: ['UInt16', '0'],
	[VescDataType.UInt32]: ['UInt32', '0'],
};
