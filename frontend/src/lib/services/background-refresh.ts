export class BackgroundRefresh {
	private pending?: Promise<void>;
	private running = false;
	private timer?: ReturnType<typeof setTimeout>;

	constructor(
		private readonly task: () => Promise<void>,
		private readonly intervalMs: number,
	) {}

	start(): void {
		if (this.running) return;
		this.running = true;
		void this.refresh();
	}

	stop(): void {
		this.running = false;
		if (this.timer !== undefined) clearTimeout(this.timer);
		this.timer = undefined;
	}

	refresh(): Promise<void> {
		if (!this.running) return Promise.resolve();
		if (this.pending) return this.pending;
		if (this.timer !== undefined) clearTimeout(this.timer);
		this.timer = undefined;

		const pending = Promise.resolve()
			.then(this.task)
			.finally(() => {
				if (this.pending === pending) this.pending = undefined;
				if (this.running) this.schedule();
			});
		this.pending = pending;
		return pending;
	}

	private schedule(): void {
		if (this.timer !== undefined) clearTimeout(this.timer);
		this.timer = setTimeout(() => void this.refresh(), this.intervalMs);
	}
}
