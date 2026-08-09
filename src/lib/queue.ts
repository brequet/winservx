import type { QueueAction, ServiceStartType } from '$lib/tauri/bindings';

export type { QueueAction, QueueTask, QueueTaskStatus } from '$lib/tauri/bindings';

/** Actions a row can request from the backend; never a startup-type change. */
export type ServiceAction = Exclude<QueueAction, { setStartType: ServiceStartType }>;

export const ACTION_LABEL: Record<ServiceAction, string> = {
	start: 'start',
	stop: 'stop',
	restart: 'restart',
	forceStart: 'force start'
};

/** The runtime action of a task, or null for startup-type changes. */
export function runtimeAction(action: QueueAction): ServiceAction | null {
	return typeof action === 'string' ? action : null;
}

/** Label for a task, including the target startup type when relevant. */
export function actionLabel(action: QueueAction): string {
	if (typeof action === 'string') return ACTION_LABEL[action];
	return `set startup type: ${action.setStartType}`;
}
