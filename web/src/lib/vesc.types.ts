import z from 'zod';

export const typedNumber = (type: VescDataType) => z.number().describe(type);
export const typedLiteral = (value: number, type: VescDataType) => z.literal(value).describe(type);

export enum VescDataType {
	UInt8 = 'uint8',
	Int8 = 'int8',
	UInt16 = 'uint16',
	Int16 = 'int16',
	UInt32 = 'uint32',
	Int32 = 'int32',
	Double16 = 'double16',
	Double32 = 'double32',
	Float32 = 'float32',
	String = 'string'
}
