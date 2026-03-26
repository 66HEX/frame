import { confirm, message, open } from '@tauri-apps/plugin-dialog';

export interface NativeDialogFilter {
	name: string;
	extensions: string[];
}

export interface NativeFileDialogOptions {
	title?: string;
	filters?: NativeDialogFilter[];
	multiple?: boolean;
	directory?: boolean;
	defaultPath?: string;
	recursive?: boolean;
}

export async function openNativeFileDialog(
	options: NativeFileDialogOptions = {}
): Promise<string | string[] | null> {
	const result = await open({
		title: options.title,
		filters: options.filters,
		multiple: options.multiple,
		directory: options.directory,
		defaultPath: options.defaultPath,
		recursive: options.recursive
	});

	if (!result) {
		return null;
	}

	if (options.multiple) {
		return Array.isArray(result) ? result : [result];
	}

	return Array.isArray(result) ? (result[0] ?? null) : result;
}

export interface NativeAskDialogOptions {
	title?: string;
	message: string;
	kind?: 'info' | 'warning' | 'error' | 'question';
	okLabel?: string;
	cancelLabel?: string;
}

export async function askNativeDialog(options: NativeAskDialogOptions): Promise<boolean> {
	const kind = options.kind === 'question' ? 'info' : options.kind;

	if (options.cancelLabel) {
		return confirm(options.message, {
			title: options.title,
			kind,
			okLabel: options.okLabel,
			cancelLabel: options.cancelLabel
		});
	}

	await message(options.message, {
		title: options.title,
		kind,
		buttons: options.okLabel ? { ok: options.okLabel } : 'Ok'
	});

	return true;
}
