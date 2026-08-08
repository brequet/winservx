export type Result<T, E> = Ok<T> | Err<E>;

export interface Ok<T> {
	readonly ok: true;
	readonly value: T;
}

export interface Err<E> {
	readonly ok: false;
	readonly error: E;
}

export function ok<T, E = never>(value: T): Result<T, E> {
	return { ok: true, value };
}

export function err<E, T = never>(error: E): Result<T, E> {
	return { ok: false, error };
}

export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
	return result.ok;
}

export function isErr<T, E>(result: Result<T, E>): result is Err<E> {
	return !result.ok;
}

export function match<T, E, R>(
	result: Result<T, E>,
	handlers: { ok: (value: T) => R; err: (error: E) => R }
): R {
	return result.ok ? handlers.ok(result.value) : handlers.err(result.error);
}

export function map<T, E, U>(result: Result<T, E>, fn: (value: T) => U): Result<U, E> {
	return result.ok ? ok(fn(result.value)) : result;
}

export function mapError<T, E, F>(result: Result<T, E>, fn: (error: E) => F): Result<T, F> {
	return result.ok ? result : err(fn(result.error));
}

export function andThen<T, E, U, F>(
	result: Result<T, E>,
	fn: (value: T) => Result<U, F>
): Result<U, E | F> {
	return result.ok ? fn(result.value) : result;
}

export function unwrapOr<T, E>(result: Result<T, E>, fallback: T): T {
	return result.ok ? result.value : fallback;
}
