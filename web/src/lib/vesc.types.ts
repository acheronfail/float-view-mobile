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

export type VescDataTypeToTsType<T extends VescDataType> = {
	[VescDataType.UInt8]: number;
	[VescDataType.Int8]: number;
	[VescDataType.UInt16]: number;
	[VescDataType.Int16]: number;
	[VescDataType.UInt32]: number;
	[VescDataType.Int32]: number;
	[VescDataType.Double16]: number;
	[VescDataType.Double32]: number;
	[VescDataType.Float32]: number;
	[VescDataType.String]: string;
}[T];
