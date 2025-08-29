import z from 'zod';

export const createApiResult = <T extends z.ZodTypeAny>(dataSchema: T) =>
	z.discriminatedUnion('kind', [
		z.object({
			kind: z.literal('success'),
			data: dataSchema
		}),
		z.object({
			kind: z.literal('error'),
			data: z.object({
				message: z.string()
			})
		})
	]);

export const BleScanResult = createApiResult(
	z.object({ name: z.string(), id: z.string() }).array()
);

export const BleConnectResult = createApiResult(z.object({ status: z.string() }));
