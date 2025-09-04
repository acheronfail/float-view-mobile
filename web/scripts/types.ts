// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Schema = any;

export type Enums = Record<
	string,
	{
		[key: string]: number | string;
	}
>;

export function enumVariants(e: Enums[string]) {
	return Object.values(e)
		.filter((v): v is number => typeof v === 'number' && !Number.isNaN(v))
		.map((n): [string, number] => [e[n] as string, n]);
}
