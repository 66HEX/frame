#!/usr/bin/env node
/**
 * Fetches Real-ESRGAN binaries for the current platform/architecture
 * and installs them into src-tauri/binaries.
 * Usage: node scripts/setup-upscaler.cjs [--force]
 */
const os = require('os');
const path = require('path');
const fs = require('fs');
const fsp = require('fs/promises');
const { Readable } = require('stream');
const { pipeline } = require('stream/promises');
const extract = require('extract-zip');

if (typeof fetch !== 'function') {
	console.error('This script requires Node.js 18 or newer (built-in fetch API).');
	process.exit(1);
}

const force = process.argv.includes('--force');

// Parse --arch and --platform from arguments
const archArgIndex = process.argv.indexOf('--arch');
const archOverride = archArgIndex !== -1 ? process.argv[archArgIndex + 1] : null;

const platformArgIndex = process.argv.indexOf('--platform');
const platformOverride = platformArgIndex !== -1 ? process.argv[platformArgIndex + 1] : null;

const repoRoot = path.resolve(__dirname, '..');
const BIN_DIR = path.join(repoRoot, 'src-tauri', 'binaries');
const TMP_ROOT_PREFIX = path.join(os.tmpdir(), 'frame-upscaler-');

const REPO_BASE = 'https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0';

const TARGETS = {
	darwin: {
		x64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-macos.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan',
			dest: 'realesrgan-ncnn-vulkan-x86_64-apple-darwin'
		},
		arm64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-macos.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan',
			dest: 'realesrgan-ncnn-vulkan-aarch64-apple-darwin'
		},
		// Support for matrix arch aliases
		x86_64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-macos.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan',
			dest: 'realesrgan-ncnn-vulkan-x86_64-apple-darwin'
		},
		aarch64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-macos.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan',
			dest: 'realesrgan-ncnn-vulkan-aarch64-apple-darwin'
		}
	},
	linux: {
		x64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-ubuntu.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan',
			dest: 'realesrgan-ncnn-vulkan-x86_64-unknown-linux-gnu'
		},
		arm64: {
			// No official prebuilt binary for Linux ARM64 in v0.2.0
			manualBuild: true
		},
		x86_64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-ubuntu.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan',
			dest: 'realesrgan-ncnn-vulkan-x86_64-unknown-linux-gnu'
		}
	},
	win32: {
		x64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-windows.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan.exe',
			dest: 'realesrgan-ncnn-vulkan-x86_64-pc-windows-msvc.exe'
		},
		x86_64: {
			url: `${REPO_BASE}/realesrgan-ncnn-vulkan-20220424-windows.zip`,
			zipEntryName: 'realesrgan-ncnn-vulkan.exe',
			dest: 'realesrgan-ncnn-vulkan-x86_64-pc-windows-msvc.exe'
		}
	}
};

async function main() {
	const platform = platformOverride || os.platform();
	const arch = archOverride || os.arch();
	const target = TARGETS[platform]?.[arch];

	if (!target) {
		console.error(`Unsupported platform or architecture: ${platform}/${arch}.`);
		process.exit(1);
	}

	if (target.manualBuild) {
		console.warn(`
No prebuilt binary available for ${platform}/${arch}.`);
		console.warn(`You must build 'realesrgan-ncnn-vulkan' from source manually and place it at:`);
		console.warn(`  src-tauri/binaries/realesrgan-ncnn-vulkan-aarch64-unknown-linux-gnu
`);
		return;
	}

	await fsp.mkdir(BIN_DIR, { recursive: true });
	const destination = path.join(BIN_DIR, target.dest);

	if (!force && (await fileExists(destination))) {
		console.log(`Skipping ${target.dest} (already exists). Use --force to re-download.`);
		return;
	}

	const tmpDir = await fsp.mkdtemp(TMP_ROOT_PREFIX);
	const zipPath = path.join(tmpDir, 'upscaler.zip');

	try {
		console.log(`Downloading upscaler from ${target.url}...`);
		await downloadFile(target.url, zipPath);

		console.log('Extracting...');
		await extract(zipPath, { dir: tmpDir });

		const binaryPath = await findFile(tmpDir, target.zipEntryName);

		if (!binaryPath) {
			throw new Error(`Could not locate ${target.zipEntryName} in downloaded archive.`);
		}

		await fsp.copyFile(binaryPath, destination);

		if (process.platform !== 'win32') {
			await fsp.chmod(destination, 0o755);
		}

		console.log(`Placed ${path.basename(destination)}.`);
	} finally {
		await safeRm(tmpDir);
	}
}

async function downloadFile(url, destination) {
	const response = await fetch(url);
	if (!response.ok || !response.body) {
		throw new Error(`Failed to download ${url}: ${response.status} ${response.statusText}`);
	}
	await pipeline(Readable.fromWeb(response.body), fs.createWriteStream(destination));
}

async function findFile(dir, targetName) {
	const entries = await fsp.readdir(dir, { withFileTypes: true });
	for (const entry of entries) {
		const entryPath = path.join(dir, entry.name);
		if (entry.isFile() && entry.name === targetName) {
			return entryPath;
		}
		if (entry.isDirectory()) {
			const nested = await findFile(entryPath, targetName);
			if (nested) {
				return nested;
			}
		}
	}
	return null;
}

async function fileExists(targetPath) {
	try {
		await fsp.access(targetPath);
		return true;
	} catch {
		return false;
	}
}

async function safeRm(targetPath) {
	await fsp.rm(targetPath, { recursive: true, force: true });
}

main().catch((err) => {
	console.error('Failed to setup upscaler binary.');
	console.error(err.message);
	process.exit(1);
});
