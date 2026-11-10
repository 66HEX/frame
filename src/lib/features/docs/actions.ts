function requireCopyableText(text: string): string {
	if (!text) throw new Error('There is no text to copy.');
	return text;
}

async function copyText(text: string): Promise<void> {
	requireCopyableText(text);

	try {
		if (navigator.clipboard?.writeText) {
			await navigator.clipboard.writeText(text);
			return;
		}
	} catch {
		// Fall back to the document copy command when the Clipboard API is unavailable or denied.
	}

	const textarea = document.createElement('textarea');
	const activeElement =
		document.activeElement instanceof HTMLElement ? document.activeElement : null;
	textarea.value = text;
	textarea.readOnly = true;
	textarea.style.position = 'fixed';
	textarea.style.inset = '0 auto auto -9999px';
	textarea.style.opacity = '0';
	document.body.append(textarea);
	textarea.select();

	let copied: boolean;
	try {
		copied = document.execCommand('copy');
	} finally {
		textarea.remove();
		activeElement?.focus();
	}

	if (!copied) throw new Error('The browser did not allow copying.');
}

async function loadRawMarkdown(rawPath: string): Promise<string> {
	const response = await fetch(rawPath, {
		headers: { Accept: 'text/markdown, text/plain;q=0.9' }
	});

	if (!response.ok) {
		throw new Error(`Unable to load Markdown (${response.status}).`);
	}

	return requireCopyableText(await response.text());
}

async function copyTextWhenReady(textPromise: Promise<string>): Promise<void> {
	try {
		if (navigator.clipboard?.write && typeof ClipboardItem !== 'undefined') {
			const item = new ClipboardItem({
				'text/plain': textPromise.then(
					(text) => new Blob([requireCopyableText(text)], { type: 'text/plain' })
				)
			});

			await navigator.clipboard.write([item]);
			return;
		}
	} catch {
		// Fall through to the text and document-copy paths for older or restricted browsers.
	}

	await copyText(await textPromise);
}

export function copyRawMarkdown(rawPath: string): Promise<void> {
	// WebKit expires user activation after an awaited fetch. Passing the pending text to
	// ClipboardItem lets clipboard.write start synchronously inside the click handler.
	return copyTextWhenReady(loadRawMarkdown(rawPath));
}
