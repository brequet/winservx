import { commands } from '../tauri/bindings';
import { err, ok, type Result } from '../result';
import { normalizeError, type ApiError } from './errors';

export async function isElevated(): Promise<Result<boolean, ApiError>> {
	try {
		return ok(await commands.isElevated());
	} catch (e) {
		return err(normalizeError(e));
	}
}

/** Relaunches the app elevated; the current process exits on success. */
export async function relaunchAsElevated(): Promise<Result<null, ApiError>> {
	try {
		return ok(await commands.relaunchAsElevated());
	} catch (e) {
		return err(normalizeError(e));
	}
}
