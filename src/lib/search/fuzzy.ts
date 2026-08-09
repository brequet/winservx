/**
 * Minimal fuzzy search engine.
 *
 * Matches are subsequences of the target text, scored so the most coherent
 * hits rank first: exact and prefix matches beat infix runs, contiguous
 * runs beat scattered hits, and shorter gaps beat longer ones. Separators
 * (`-`, `_`, ` `, …) are transparent to matching, so `sqlsrv` finds
 * "SQL Server". Matching is case-insensitive.
 */

const BONUS_START = 30;
const BONUS_BOUNDARY = 20;
const BONUS_CONSECUTIVE = 15;
const BONUS_PREFIX = 25;
const BONUS_EXACT = 25;
const GAP_PENALTY = 2;

interface CleanChar {
	ch: string;
	/** True when the char starts a word in the original text. */
	boundary: boolean;
}

/**
 * Scores how well `query` matches `text` as a fuzzy subsequence.
 * Returns `null` when there is no match, `0` for an empty query.
 */
export function fuzzyScore(query: string, text: string): number | null {
	const needle = query.toLowerCase().replace(/[^a-z0-9]/g, '');
	if (needle.length === 0) return 0;
	const chars = clean(text);
	if (chars.length < needle.length) return null;

	let score = 0;
	let matched = 0;
	let previous = -1;
	let first = -1;
	let gaps = 0;

	for (let i = 0; i < chars.length && matched < needle.length; i++) {
		if (chars[i].ch !== needle[matched]) continue;
		if (matched === 0) first = i;
		if (previous !== -1) gaps += i - previous - 1;
		score += 1;
		if (i === 0) score += BONUS_START;
		else if (chars[i].boundary) score += BONUS_BOUNDARY;
		if (previous === i - 1) score += BONUS_CONSECUTIVE;
		previous = i;
		matched++;
	}

	if (matched < needle.length) return null;
	if (first === 0) score += BONUS_PREFIX;
	if (chars.length === needle.length) score += BONUS_EXACT;
	return score - gaps * GAP_PENALTY;
}

function clean(text: string): CleanChar[] {
	const chars: CleanChar[] = [];
	let alnum = false;
	for (const ch of text.toLowerCase()) {
		if (/[a-z0-9]/.test(ch)) {
			chars.push({ ch, boundary: !alnum });
			alnum = true;
		} else {
			alnum = false;
		}
	}
	return chars;
}
