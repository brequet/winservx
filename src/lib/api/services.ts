import { commands, type ServiceInfo, type ServiceStartType } from '../tauri/bindings';
import { err, ok, type Result } from '../result';
import type { ServiceAction } from '../queue';
import { normalizeError, type ApiError } from './errors';

const ACTION_COMMAND: Record<ServiceAction, (name: string) => Promise<null>> = {
	start: (name) => commands.startService(name),
	stop: (name) => commands.stopService(name),
	restart: (name) => commands.restartService(name),
	forceStart: (name) => commands.forceStartService(name)
};

export async function loadServices(): Promise<Result<ServiceInfo[], ApiError>> {
	try {
		return ok(await commands.getServices());
	} catch (e) {
		return err(normalizeError(e));
	}
}

export async function runServiceAction(
	name: string,
	action: ServiceAction
): Promise<Result<null, ApiError>> {
	try {
		return ok(await ACTION_COMMAND[action](name));
	} catch (e) {
		return err(normalizeError(e));
	}
}

export async function updateStartupType(
	name: string,
	startType: ServiceStartType
): Promise<Result<null, ApiError>> {
	try {
		return ok(await commands.updateStartupType(name, startType));
	} catch (e) {
		return err(normalizeError(e));
	}
}
