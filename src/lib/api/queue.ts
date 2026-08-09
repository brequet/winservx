import { commands, events } from '../tauri/bindings';
import type { QueueAction, QueueTask, QueueTaskUpdated } from '../tauri/bindings';
import { err, ok, type Result } from '../result';
import { normalizeError, type ApiError } from './errors';

export async function enqueueAction(
	serviceName: string,
	action: QueueAction
): Promise<Result<number, ApiError>> {
	try {
		return ok(await commands.enqueueAction(action, serviceName));
	} catch (e) {
		return err(normalizeError(e));
	}
}

export async function loadQueue(): Promise<Result<QueueTask[], ApiError>> {
	try {
		return ok(await commands.getQueue());
	} catch (e) {
		return err(normalizeError(e));
	}
}

export async function dismissTask(id: number): Promise<Result<null, ApiError>> {
	try {
		return ok(await commands.dismissTask(id));
	} catch (e) {
		return err(normalizeError(e));
	}
}

export interface QueueHandlers {
	onTaskUpdated: (event: QueueTaskUpdated) => void;
}

export async function subscribeToQueue(handlers: QueueHandlers): Promise<Array<() => void>> {
	return [await events.queueTaskUpdated.listen((e) => handlers.onTaskUpdated(e.payload))];
}
