export function readPreference(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}
export function writePreference(key: string, value: string) {
	try {
		localStorage.setItem(key, value);
	} catch {
		/* The current session still uses the selected value. */
	}
}
