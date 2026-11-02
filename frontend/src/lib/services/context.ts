import { getContext, setContext } from 'svelte';

import { ArcadiaNotifications } from '$lib/arcadia-notifications.svelte';
import type { ArcadiaServices } from '$lib/services/arcadia-services';

const arcadiaServicesKey = Symbol('arcadia-services');
const arcadiaNotificationsKey = Symbol('arcadia-notifications');

export function provideArcadiaServices(
	services: ArcadiaServices,
): ArcadiaServices {
	setContext(arcadiaServicesKey, services);
	return services;
}

export function getArcadiaServices(): ArcadiaServices {
	const services = getContext<ArcadiaServices>(arcadiaServicesKey);
	if (!services) throw new Error('Arcadia services are unavailable.');
	return services;
}

export function provideArcadiaNotifications(
	notifications: ArcadiaNotifications,
): ArcadiaNotifications {
	setContext(arcadiaNotificationsKey, notifications);
	return notifications;
}

export function getArcadiaNotifications(): ArcadiaNotifications {
	const notifications = getContext<ArcadiaNotifications>(
		arcadiaNotificationsKey,
	);
	if (!notifications) throw new Error('Arcadia notifications are unavailable.');
	return notifications;
}
