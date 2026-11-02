import { SvelteMap } from 'svelte/reactivity';

export type ArcadiaNoticeKind = 'success' | 'error' | 'info';

export interface ArcadiaNotice {
	id: number;
	kind: ArcadiaNoticeKind;
	message: string;
}

const successDurationMs = 4_000;
const infoDurationMs = 4_500;
const errorDurationMs = 7_000;
const maxVisibleNotices = 4;

export class ArcadiaNotifications {
	notices = $state<ArcadiaNotice[]>([]);

	private nextId = 0;
	private readonly timers = new SvelteMap<number, number>();

	success(message: string): void {
		this.push('success', message, successDurationMs);
	}

	error(message: string): void {
		this.push('error', message, errorDurationMs);
	}

	info(message: string): void {
		this.push('info', message, infoDurationMs);
	}

	dismiss(id: number): void {
		this.clearTimer(id);
		this.notices = this.notices.filter((notice) => notice.id !== id);
	}

	destroy(): void {
		for (const id of [...this.timers.keys()]) this.clearTimer(id);
		this.notices = [];
	}

	private push(
		kind: ArcadiaNoticeKind,
		message: string,
		durationMs: number,
	): void {
		const normalized = message.trim();
		if (!normalized) return;

		const existing = this.notices.find(
			(notice) => notice.kind === kind && notice.message === normalized,
		);
		if (existing) {
			this.scheduleDismissal(existing.id, durationMs);
			return;
		}

		const notice = {
			id: ++this.nextId,
			kind,
			message: normalized,
		} satisfies ArcadiaNotice;
		const nextNotices = [...this.notices, notice];
		while (nextNotices.length > maxVisibleNotices) {
			const removed = nextNotices.shift();
			if (removed) this.clearTimer(removed.id);
		}
		this.notices = nextNotices;
		this.scheduleDismissal(notice.id, durationMs);
	}

	private scheduleDismissal(id: number, durationMs: number): void {
		this.clearTimer(id);
		if (typeof window === 'undefined') return;
		this.timers.set(
			id,
			window.setTimeout(() => this.dismiss(id), durationMs),
		);
	}

	private clearTimer(id: number): void {
		const timer = this.timers.get(id);
		if (timer !== undefined && typeof window !== 'undefined')
			window.clearTimeout(timer);
		this.timers.delete(id);
	}
}
