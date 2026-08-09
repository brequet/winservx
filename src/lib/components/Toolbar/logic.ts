export type EscapeAction = 'clear' | 'blur-search' | 'close-columns' | 'none';

export interface EscapeInput {
	focusedInSearch: boolean;
	hasValue: boolean;
	columnsOpen: boolean;
	typingTarget: boolean;
}

/** What Escape should do, given where focus is and what is open. */
export function escapeAction(input: EscapeInput): EscapeAction {
	if (input.focusedInSearch) {
		return input.hasValue ? 'clear' : 'blur-search';
	}
	if (input.columnsOpen || input.typingTarget) return 'close-columns';
	return input.hasValue ? 'clear' : 'none';
}
