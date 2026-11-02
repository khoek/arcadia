export class AsyncCommitQueue {
	private epoch = 0;
	private pendingCount = 0;
	private tail = Promise.resolve();

	constructor(private readonly onError: (error: unknown) => void) {}

	get busy(): boolean {
		return this.pendingCount > 0;
	}

	enqueue(commit: () => Promise<void>): Promise<void> {
		const epoch = this.epoch;
		this.pendingCount += 1;
		const run = async (): Promise<void> => {
			if (epoch !== this.epoch) return;
			try {
				await commit();
			} catch (error) {
				this.onError(error);
				throw error;
			} finally {
				if (epoch === this.epoch) this.pendingCount -= 1;
			}
		};
		const operation = this.tail.then(run, run);
		this.tail = operation.then(
			() => undefined,
			() => undefined,
		);
		// Callers can await the returned operation, while ignored operations still
		// count as handled and keep the queue moving after an error.
		void operation.catch(() => undefined);
		return operation;
	}

	whenIdle(): Promise<void> {
		return this.tail;
	}

	reset(): void {
		this.epoch += 1;
		this.pendingCount = 0;
		this.tail = Promise.resolve();
	}
}
