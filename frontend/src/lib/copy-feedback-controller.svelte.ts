import { copyFeedbackDurationMs } from '$lib/copy-feedback';

/**
 * Owns the short-lived visual state for copy actions.
 *
 * A new action claims the feedback slot synchronously.  Any clipboard/API
 * work belonging to an older action is then unable to put its result back on
 * screen because its sequence is no longer current.
 */
export class CopyFeedbackController<T> {
	active = $state<T | null>(null);
	sequence = $state(0);

	private destroyed = false;
	private timeout: number | undefined;

	begin(target: T): number {
		if (this.destroyed) return this.sequence;
		this.sequence += 1;
		this.clearTimer();
		this.active = target;
		return this.sequence;
	}

	reset(): void {
		if (this.destroyed) return;
		this.sequence += 1;
		this.clearTimer();
		this.active = null;
	}

	isCurrent(sequence: number): boolean {
		return !this.destroyed && sequence === this.sequence;
	}

	finish(sequence: number): void {
		if (!this.isCurrent(sequence)) return;
		this.clearTimer();
		this.sequence += 1;
		const feedbackSequence = this.sequence;
		if (typeof window === 'undefined') return;
		this.timeout = window.setTimeout(() => {
			if (!this.isCurrent(feedbackSequence)) return;
			this.timeout = undefined;
			this.active = null;
		}, copyFeedbackDurationMs);
	}

	cancel(sequence: number): void {
		if (!this.isCurrent(sequence)) return;
		this.clearTimer();
		this.active = null;
	}

	destroy(): void {
		if (this.destroyed) return;
		this.destroyed = true;
		this.clearTimer();
		this.active = null;
	}

	private clearTimer(): void {
		if (this.timeout !== undefined && typeof window !== 'undefined')
			window.clearTimeout(this.timeout);
		this.timeout = undefined;
	}
}
