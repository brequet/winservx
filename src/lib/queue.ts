import type { ServiceStartType } from '$lib/tauri/bindings';
import type { ServiceAction } from './api/services';

/** A row/queue action, including non-runtime changes like startup type. */
export type QueueAction = ServiceAction | 'setStartType';

export interface QueueItem {
	id: number;
	serviceName: string;
	action: QueueAction;
	/** Target startup type, present when `action === 'setStartType'`. */
	startType?: ServiceStartType;
	status: 'inFlight' | 'success' | 'failed';
	/** Formatted error message, present when `status === 'failed'`. */
	error?: string;
}

export const ACTION_LABEL: Record<QueueAction, string> = {
	start: 'start',
	stop: 'stop',
	restart: 'restart',
	forceStart: 'force start',
	setStartType: 'set startup type'
};

/** Label for a queue item, including the target startup type when relevant. */
export function actionLabel(item: Pick<QueueItem, 'action' | 'startType'>): string {
	return item.action === 'setStartType' && item.startType
		? `${ACTION_LABEL.setStartType}: ${item.startType}`
		: ACTION_LABEL[item.action];
}
