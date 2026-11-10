export type KeyboardShortcut = {
	key: string;
	shiftKey: boolean;
	display: string;
	ariaKeyShortcuts: string;
};

export type KeyboardShortcutMatchOptions = {
	allowEditableTarget?: boolean;
};

export const keyboardShortcuts = {
	sidebar: {
		key: 'b',
		shiftKey: false,
		display: '⌘B',
		ariaKeyShortcuts: 'Meta+B Control+B'
	},
	theme: {
		key: 'l',
		shiftKey: true,
		display: '⌘⇧L',
		ariaKeyShortcuts: 'Meta+Shift+L Control+Shift+L'
	},
	search: {
		key: 'k',
		shiftKey: false,
		display: '⌘K',
		ariaKeyShortcuts: 'Meta+K Control+K'
	}
} as const satisfies Record<string, KeyboardShortcut>;

function isEditableTarget(target: EventTarget | null) {
	if (!(target instanceof HTMLElement)) return false;

	return Boolean(
		target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])')
	);
}

export function matchesKeyboardShortcut(
	event: KeyboardEvent,
	shortcut: KeyboardShortcut,
	options: KeyboardShortcutMatchOptions = {}
) {
	return (
		!event.defaultPrevented &&
		!event.repeat &&
		!event.isComposing &&
		(event.metaKey || event.ctrlKey) &&
		!event.altKey &&
		event.shiftKey === shortcut.shiftKey &&
		event.key.toLowerCase() === shortcut.key &&
		(options.allowEditableTarget || !isEditableTarget(event.target))
	);
}
