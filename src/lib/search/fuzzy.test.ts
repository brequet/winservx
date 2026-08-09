import { describe, expect, it } from 'vitest';
import { fuzzyScore } from './fuzzy';

/** Scores a known match; fails the test when the query does not match. */
function score(query: string, text: string): number {
	const result = fuzzyScore(query, text);
	if (result === null) throw new Error(`expected "${query}" to match "${text}"`);
	return result;
}

describe('fuzzyScore', () => {
	it('scores an empty query as a match with zero score', () => {
		expect(fuzzyScore('', 'anything')).toBe(0);
	});

	it('matches case-insensitively', () => {
		expect(score('SQL', 'sql')).toBe(score('sql', 'SQL'));
	});

	it('returns null when the query does not match', () => {
		expect(fuzzyScore('xyz', 'abc')).toBeNull();
	});

	it('requires the whole query as a subsequence', () => {
		expect(fuzzyScore('sqlx', 'SQL Server')).toBeNull();
	});

	it('ranks exact matches above prefix matches', () => {
		expect(score('sql', 'SQL')).toBeGreaterThan(score('sql', 'SQL Server'));
	});

	it('ranks prefix matches above infix matches', () => {
		expect(score('sql', 'SQL Server')).toBeGreaterThan(score('sql', 'MSSQLSERVER'));
	});

	it('ranks consecutive runs above scattered matches', () => {
		expect(score('sql', 'sqlabc')).toBeGreaterThan(score('sql', 'sq1x2l'));
	});

	it('penalizes gaps between matched characters', () => {
		expect(score('ad', 'abcdef')).toBeGreaterThan(score('ad', 'abcxdef'));
	});

	it('is transparent to separators in query and text', () => {
		expect(score('sql server', 'SQL Server')).toBe(score('sqlserver', 'SQL Server'));
		expect(score('sqlsrv', 'SQL Server')).toBeGreaterThan(0);
	});

	it('scores any match positively', () => {
		expect(score('s', 's')).toBeGreaterThan(0);
	});
});
