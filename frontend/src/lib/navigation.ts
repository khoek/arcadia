import { CreditCard, House, LockKeyhole, ShieldCheck } from '@lucide/svelte';
import { base } from '$app/paths';

export const navigationItems = [
	{
		href: `${base}/home`,
		label: 'Home',
		icon: House,
		section: null,
	},
	{
		href: `${base}/authenticator`,
		label: 'Authenticator',
		icon: ShieldCheck,
		section: 'Vault',
	},
	{
		href: `${base}/vault`,
		label: 'Passwords',
		icon: LockKeyhole,
		section: 'Vault',
	},
	{
		href: `${base}/cards`,
		label: 'Cards',
		icon: CreditCard,
		section: 'Vault',
	},
] as const;

export type NavigationItem = (typeof navigationItems)[number];

export function navigationItemIsActive(
	pathname: string,
	item: NavigationItem,
): boolean {
	return pathname.startsWith(item.href);
}
