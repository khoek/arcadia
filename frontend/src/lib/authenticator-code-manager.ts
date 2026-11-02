import type {
	AuthenticatorCodeBatch,
	AuthenticatorCodePair,
	AuthenticatorCodePeriod,
} from '$lib/authenticator';
import type { ArcadiaApi } from '$lib/arcadia';

export class AuthenticatorCodeManager {
	private batch?: AuthenticatorCodeBatch;
	private clockOffset = 0;
	private errorListener?: (error: unknown) => void;
	private listener?: (batch: AuthenticatorCodeBatch) => void;
	private refreshTimer?: ReturnType<typeof setTimeout>;
	private running = false;
	private retryDelayMs = 1_000;

	constructor(private readonly api: ArcadiaApi) {}

	async start(
		listener: (batch: AuthenticatorCodeBatch) => void,
		errorListener?: (error: unknown) => void,
	): Promise<void> {
		this.listener = listener;
		this.errorListener = errorListener;
		this.running = true;
		try {
			await this.refresh();
		} catch (error) {
			this.scheduleRetry();
			throw error;
		}
	}

	stop(): void {
		this.running = false;
		this.listener = undefined;
		this.errorListener = undefined;
		if (this.refreshTimer !== undefined) clearTimeout(this.refreshTimer);
	}

	async refresh(): Promise<void> {
		const batch = await this.api.getAuthenticatorCodeBatch();
		this.batch = batch;
		this.clockOffset = batch.serverTime - Date.now();
		this.retryDelayMs = 1_000;
		this.listener?.(batch);
		if (this.running) this.scheduleRefresh(batch);
	}

	codesAt(clientTime: number): Map<string, AuthenticatorCodePeriod> {
		const serverTime = clientTime + this.clockOffset;
		return new Map(
			(this.batch?.codes ?? []).flatMap((pair) => {
				const code = periodAt(pair, serverTime);
				return code ? [[pair.authenticatorId, code]] : [];
			}),
		);
	}

	progressAt(clientTime: number): number {
		const serverTime = clientTime + this.clockOffset;
		const pair = this.batch?.codes[0];
		const period = pair && periodAt(pair, serverTime);
		return period
			? Math.max(
					0,
					Math.min(
						1,
						(serverTime - period.validFrom) /
							(period.validUntil - period.validFrom),
					),
				)
			: 0;
	}

	private scheduleRefresh(batch: AuthenticatorCodeBatch): void {
		if (this.refreshTimer !== undefined) clearTimeout(this.refreshTimer);
		const nextBoundary = Math.min(
			...batch.codes.map((pair) => pair.current.validUntil),
		);
		const delay = Number.isFinite(nextBoundary)
			? Math.max(100, nextBoundary - batch.serverTime + 100)
			: 30_000;
		this.refreshTimer = setTimeout(
			() => void this.refreshInBackground(),
			delay,
		);
	}

	private async refreshInBackground(): Promise<void> {
		try {
			await this.refresh();
		} catch (error) {
			this.errorListener?.(error);
			this.scheduleRetry();
		}
	}

	private scheduleRetry(): void {
		if (!this.running) return;
		if (this.refreshTimer !== undefined) clearTimeout(this.refreshTimer);
		this.refreshTimer = setTimeout(
			() => void this.refreshInBackground(),
			this.retryDelayMs,
		);
		this.retryDelayMs = Math.min(this.retryDelayMs * 2, 15_000);
	}
}

function periodAt(
	pair: AuthenticatorCodePair,
	serverTime: number,
): AuthenticatorCodePeriod | undefined {
	if (serverTime < pair.current.validUntil) return pair.current;
	if (serverTime < pair.next.validUntil) return pair.next;
	return undefined;
}
