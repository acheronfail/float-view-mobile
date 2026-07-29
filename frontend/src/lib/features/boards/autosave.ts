export class SerialSaveQueue {
	private pending = Promise.resolve();

	enqueue(action: () => Promise<void>): void {
		this.pending = this.pending.then(action, action);
	}

	async flush(): Promise<void> {
		await this.pending;
	}
}

export function validSeriesCount(value: number | undefined): value is number {
	return Number.isInteger(value) && Number(value) >= 1 && Number(value) <= 36;
}

export function validParallelCount(value: number | undefined): value is number {
	return Number.isInteger(value) && Number(value) >= 1 && Number(value) <= 20;
}

export function normalizedCustomName(value: string): string | null {
	return value.trim() || null;
}
