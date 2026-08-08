import type { ServiceAction } from './api/services';

export interface QueueItem {
	id: number;
	serviceName: string;
	action: ServiceAction;
	status: 'inFlight' | 'success' | 'failed';
	/** Formatted error message, present when `status === 'failed'`. */
	error?: string;
}

export const ACTION_LABEL: Record<ServiceAction, string> = {
	start: 'start',
	stop: 'stop',
	restart: 'restart',
	forceStart: 'force start'
};
