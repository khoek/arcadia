import { CreditCard, House, LockKeyhole, ShieldCheck } from '@lucide/svelte';

export const navigationItems = [
	{
		href: '/home',
		label: 'Home',
		icon: House,
		section: null,
	},
	{
		href: '/authenticator',
		label: 'Authenticator',
		icon: ShieldCheck,
		section: 'Vault',
	},
	{
		href: '/vault',
		label: 'Passwords',
		icon: LockKeyhole,
		section: 'Vault',
	},
	{
		href: '/cards',
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
