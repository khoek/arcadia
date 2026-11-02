import type { ArcadiaApi } from '$lib/arcadia';
import { AuthenticatorService } from '$lib/services/authenticator-service.svelte';
import { CardService } from '$lib/services/card-service.svelte';
import { VaultService } from '$lib/services/vault-service.svelte';

export class ArcadiaServices {
	readonly authenticator: AuthenticatorService;
	readonly cards: CardService;
	readonly vault: VaultService;

	private running = false;

	constructor(api: ArcadiaApi) {
		this.authenticator = new AuthenticatorService(api);
		this.cards = new CardService(api);
		this.vault = new VaultService(api);
	}

	start(): void {
		if (this.running) return;
		this.running = true;
		this.authenticator.start();
		this.cards.start();
		this.vault.start();
		window.addEventListener('online', this.refresh);
		document.addEventListener('visibilitychange', this.refreshWhenVisible);
	}

	stop(): void {
		if (!this.running) return;
		this.running = false;
		window.removeEventListener('online', this.refresh);
		document.removeEventListener('visibilitychange', this.refreshWhenVisible);
		this.authenticator.stop();
		this.cards.stop();
		this.vault.stop();
	}

	private readonly refresh = (): void => {
		if (!this.running) return;
		void Promise.all([
			this.authenticator.refresh(),
			this.cards.refresh(),
			this.vault.refresh(),
		]);
	};

	private readonly refreshWhenVisible = (): void => {
		if (document.visibilityState === 'visible') this.refresh();
	};
}
