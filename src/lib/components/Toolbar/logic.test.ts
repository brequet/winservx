import { describe, expect, it } from 'vitest';
import { escapeAction } from './logic';

describe('escapeAction', () => {
	it('clears the query when Escape is pressed while focused in the search input', () => {
		expect(
			escapeAction({
				focusedInSearch: true,
				hasValue: true,
				columnsOpen: false,
				typingTarget: false
			})
		).toBe('clear');
	});

	it('blurs the search input when it is focused and already empty', () => {
		expect(
			escapeAction({
				focusedInSearch: true,
				hasValue: false,
				columnsOpen: false,
				typingTarget: false
			})
		).toBe('blur-search');
	});

	it('clears even when the columns menu is open and the search input is focused', () => {
		expect(
			escapeAction({
				focusedInSearch: true,
				hasValue: true,
				columnsOpen: true,
				typingTarget: false
			})
		).toBe('clear');
	});

	it('closes the columns menu when Escape is pressed outside the search input', () => {
		expect(
			escapeAction({
				focusedInSearch: false,
				hasValue: false,
				columnsOpen: true,
				typingTarget: false
			})
		).toBe('close-columns');
	});

	it('does not hijack Escape from other typing targets', () => {
		expect(
			escapeAction({
				focusedInSearch: false,
				hasValue: true,
				columnsOpen: false,
				typingTarget: true
			})
		).toBe('close-columns');
	});

	it('clears the query when Escape is pressed outside the search input', () => {
		expect(
			escapeAction({
				focusedInSearch: false,
				hasValue: true,
				columnsOpen: false,
				typingTarget: false
			})
		).toBe('clear');
	});

	it('does nothing when there is nothing to do', () => {
		expect(
			escapeAction({
				focusedInSearch: false,
				hasValue: false,
				columnsOpen: false,
				typingTarget: false
			})
		).toBe('none');
	});
});
