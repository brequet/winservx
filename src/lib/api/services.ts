import { commands, type ServiceInfo } from '../tauri/bindings';
import { err, ok, type Result } from '../result';
import { normalizeError, type ApiError } from './errors';

export async function loadServices(): Promise<Result<ServiceInfo[], ApiError>> {
	try {
		return ok(await commands.getServices());
	} catch (e) {
		return err(normalizeError(e));
	}
}
