export type ThemeMode = 'system' | 'light' | 'dark';

const STORAGE_KEY = 'winservx.theme';

function isThemeMode(value: unknown): value is ThemeMode {
	return value === 'system' || value === 'light' || value === 'dark';
}

function loadThemeMode(): ThemeMode {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (isThemeMode(stored)) return stored;
	} catch {
		// storage unavailable — fall back to system
	}
	return 'system';
}

function readOsDark(): boolean {
	try {
		return window.matchMedia('(prefers-color-scheme: dark)').matches;
	} catch {
		return false;
	}
}

export const theme = $state<{ mode: ThemeMode; osDark: boolean }>({
	mode: loadThemeMode(),
	osDark: readOsDark()
});

export function resolvedTheme(): 'light' | 'dark' {
	return theme.mode === 'system' ? (theme.osDark ? 'dark' : 'light') : theme.mode;
}

export function setThemeMode(mode: ThemeMode): void {
	theme.mode = mode;
	document.documentElement.dataset.theme = mode;
	try {
		localStorage.setItem(STORAGE_KEY, mode);
	} catch {
		// persistence unavailable — theme applies for this session only
	}
}

const media = window.matchMedia('(prefers-color-scheme: dark)');
media.addEventListener('change', (event) => (theme.osDark = event.matches));
