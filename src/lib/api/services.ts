import { commands, type ServiceError, type ServiceInfo } from '../tauri/bindings';

export async function loadServices(): Promise<ServiceInfo[]> {
	return commands.getServices();
}

function isServiceError(e: unknown): e is ServiceError {
	if (typeof e !== 'object' || e === null) return false;
	const kind = (e as { kind?: unknown }).kind;
	return kind === 'windows' || kind === 'internal';
}

export function parseServiceError(e: unknown): string {
	if (isServiceError(e)) {
		if (e.kind === 'windows') {
			return `Windows error ${e.code} (0x${e.code.toString(16).toUpperCase()}): ${e.message}`;
		}
		return e.message;
	}
	if (typeof e === 'string') return e;
	if (e instanceof Error) return e.message;
	return 'Unknown error';
}
