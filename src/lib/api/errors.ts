import type { ServiceError } from '../tauri/bindings';

export type ApiError = ServiceError | { kind: 'unknown'; message: string };

function isServiceError(e: unknown): e is ServiceError {
	if (typeof e !== 'object' || e === null) return false;
	const kind = (e as { kind?: unknown }).kind;
	return kind === 'windows' || kind === 'internal';
}

export function normalizeError(e: unknown): ApiError {
	if (isServiceError(e)) return e;
	if (typeof e === 'string') return { kind: 'unknown', message: e };
	if (e instanceof Error) return { kind: 'unknown', message: e.message };
	return { kind: 'unknown', message: 'Unknown error' };
}

export function formatApiError(error: ApiError): string {
	switch (error.kind) {
		case 'windows':
			return `Windows error ${error.code} (0x${error.code.toString(16).toUpperCase()}): ${error.message}`;
		case 'internal':
			return `Internal error: ${error.message}`;
		case 'unknown':
			return error.message;
	}
}
