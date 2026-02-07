#!/usr/bin/env node
/**
 * Fetches FFmpeg and FFprobe binaries for the current platform/architecture
 * and installs them into src-tauri/binaries.
 * Usage: node scripts/setup-binaries.cjs [--force]
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
const repoRoot = path.resolve(__dirname, '..');
const BIN_DIR = path.join(repoRoot, 'src-tauri', 'binaries');
const TMP_ROOT_PREFIX = path.join(os.tmpdir(), 'frame-binaries-');

const MARTIN_BASE = 'https://ffmpeg.martin-riedl.de/redirect/latest';
const WINDOWS_ZIP_URL =
	'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip';

const TARGETS = {
	darwin: {
		x64: createMartinTarget('macOS (Intel)', 'macos', 'amd64', 'x86_64', 'apple-darwin'),
		arm64: createMartinTarget('macOS (Apple Silicon)', 'macos', 'arm64', 'aarch64', 'apple-darwin')
	},
	linux: {
		x64: createMartinTarget('Linux x86_64', 'linux', 'amd64', 'x86_64', 'unknown-linux-gnu'),
		arm64: createMartinTarget('Linux ARM64', 'linux', 'arm64', 'aarch64', 'unknown-linux-gnu')
	},
	win32: {
		x64: {
			label: 'Windows x86_64',
			sharedArchive: {
				url: WINDOWS_ZIP_URL,
				entries: [
					{
						id: 'ffmpeg',
						expectedNames: ['ffmpeg.exe'],
						dest: 'ffmpeg-x86_64-pc-windows-msvc.exe',
						makeExecutable: false
					},
					{
						id: 'ffprobe',
						expectedNames: ['ffprobe.exe'],
						dest: 'ffprobe-x86_64-pc-windows-msvc.exe',
						makeExecutable: false
					}
				]
			}
		}
	}
};

function createMartinTarget(label, osSegment, downloadSegment, archLabel, suffix) {
	return {
		label,
		binaries: [
			{
				id: 'ffmpeg',
				url: `${MARTIN_BASE}/${osSegment}/${downloadSegment}/release/ffmpeg.zip`,
				expectedNames: ['ffmpeg'],
				dest: `ffmpeg-${archLabel}-${suffix}`,
				makeExecutable: true
			},
			{
				id: 'ffprobe',
				url: `${MARTIN_BASE}/${osSegment}/${downloadSegment}/release/ffprobe.zip`,
				expectedNames: ['ffprobe'],
				dest: `ffprobe-${archLabel}-${suffix}`,
				makeExecutable: true
			}
		]
	};
}

async function main() {
	const platform = os.platform();
	const arch = os.arch();
	const target = TARGETS[platform]?.[arch];

	if (!target) {
		console.error(`Unsupported platform or architecture: ${platform}/${arch}.`);
		process.exit(1);
	}

	await fsp.mkdir(BIN_DIR, { recursive: true });
	const tmpDir = await fsp.mkdtemp(TMP_ROOT_PREFIX);

	console.log(`Detected ${target.label}. Preparing FFmpeg binaries...`);
	try {
		if (target.sharedArchive) {
			await processSharedArchive(target.sharedArchive, tmpDir);
		} else if (target.binaries) {
			for (const entry of target.binaries) {
				await processIndividual(entry, tmpDir);
			}
		} else {
			throw new Error('Invalid target configuration. No binaries defined.');
		}
	} finally {
		await safeRm(tmpDir);
	}

	console.log('All binaries are ready in src-tauri/binaries.');
}

async function processIndividual(entry, tmpDir) {
	const destination = path.join(BIN_DIR, entry.dest);
	if (!force && (await fileExists(destination))) {
		console.log(`Skipping ${entry.dest} (already exists). Use --force to re-download.`);
		return;
	}

	const unique = `${entry.id}-${Date.now()}`;
	const zipPath = path.join(tmpDir, `${unique}.zip`);
	const extractDir = path.join(tmpDir, unique);

	console.log(`Downloading ${entry.id} from ${entry.url}...`);
	await downloadFile(entry.url, zipPath);
	await fsp.mkdir(extractDir, { recursive: true });
	await extract(zipPath, { dir: extractDir });

	const source = await findFile(extractDir, entry.expectedNames);
	if (!source) {
		throw new Error(`Could not locate ${entry.expectedNames.join(', ')} in archive ${entry.url}`);
	}

	await copyIntoPlace(source, destination, entry.makeExecutable);
	await safeRm(extractDir);
	await safeRm(zipPath);
}

async function processSharedArchive(sharedConfig, tmpDir) {
	const entries = sharedConfig.entries.map((entry) => ({
		...entry,
		destination: path.join(BIN_DIR, entry.dest)
	}));

	const needsDownload =
		force ||
		(await Promise.all(entries.map((entry) => fileExists(entry.destination)))).some(
			(exists) => !exists
		);

	if (!needsDownload) {
		console.log('Windows binaries already present. Use --force to refresh.');
		return;
	}

	const zipPath = path.join(tmpDir, 'windows.zip');
	const extractDir = path.join(tmpDir, 'windows');

	console.log(`Downloading Windows bundle from ${sharedConfig.url}...`);
	await downloadFile(sharedConfig.url, zipPath);
	await fsp.mkdir(extractDir, { recursive: true });
	await extract(zipPath, { dir: extractDir });

	for (const entry of entries) {
		if (!force && (await fileExists(entry.destination))) {
			console.log(`Skipping ${entry.dest} (already exists). Use --force to re-download.`);
			continue;
		}

		const source = await findFile(extractDir, entry.expectedNames);
		if (!source) {
			throw new Error(`Could not locate ${entry.expectedNames.join(', ')} inside Windows archive.`);
		}
		await copyIntoPlace(source, entry.destination, entry.makeExecutable);
	}

	await safeRm(extractDir);
	await safeRm(zipPath);
}

async function copyIntoPlace(source, destination, makeExecutable = true) {
	await fsp.copyFile(source, destination);
	if (makeExecutable && process.platform !== 'win32') {
		await fsp.chmod(destination, 0o755);
	}
	console.log(`Placed ${path.basename(destination)}.`);
}

async function downloadFile(url, destination) {
	const response = await fetch(url);
	if (!response.ok || !response.body) {
		throw new Error(`Failed to download ${url}: ${response.status} ${response.statusText}`);
	}
	await pipeline(Readable.fromWeb(response.body), fs.createWriteStream(destination));
}

async function findFile(dir, expectedNames) {
	const entries = await fsp.readdir(dir, { withFileTypes: true });
	for (const entry of entries) {
		const entryPath = path.join(dir, entry.name);
		if (entry.isFile() && expectedNames.includes(entry.name)) {
			return entryPath;
		}
		if (entry.isDirectory()) {
			const nested = await findFile(entryPath, expectedNames);
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
	console.error('Failed to prepare FFmpeg binaries.');
	console.error(err.message);
	process.exit(1);
});
