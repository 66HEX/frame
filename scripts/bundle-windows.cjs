#!/usr/bin/env node

const os = require('os');
const path = require('path');
const fsp = require('fs/promises');
const { spawnSync } = require('child_process');

const repoRoot = path.resolve(__dirname, '..');
const arch = os.arch();
const targetDir = process.env.CARGO_TARGET_DIR
	? path.resolve(process.env.CARGO_TARGET_DIR)
	: path.join(repoRoot, 'frame-app', 'target');

async function main() {
	if (process.platform !== 'win32') {
		console.error('scripts/bundle-windows.cjs must be run on Windows.');
		process.exit(1);
	}

	const runtimeTriple = runtimeTripleForArch(arch);
	const releaseDir = path.join(targetDir, 'release');
	const packageName = `frame-windows-${packageArchForRuntimeTriple(runtimeTriple)}`;
	const packageDir = path.join(releaseDir, packageName);
	const archivePath = path.join(releaseDir, `${packageName}.zip`);

	run(process.execPath, [path.join(repoRoot, 'scripts', 'setup-ffmpeg.cjs')]);
	run('cargo', ['build', '--manifest-path', path.join(repoRoot, 'frame-app', 'Cargo.toml'), '--release']);

	await fsp.rm(packageDir, { recursive: true, force: true });
	await fsp.mkdir(path.join(packageDir, 'binaries'), { recursive: true });

	await copyRequired(path.join(releaseDir, 'frame.exe'), path.join(packageDir, 'frame.exe'));
	for (const binary of [
		`ffmpeg-${runtimeTriple}.exe`,
		`ffprobe-${runtimeTriple}.exe`
	]) {
		await copyRequired(
			path.join(repoRoot, 'frame-app', 'resources', 'binaries', binary),
			path.join(packageDir, 'binaries', binary)
		);
	}

	await fsp.rm(archivePath, { force: true });
	compressArchive(packageDir, archivePath);

	console.log(`Created ${archivePath}`);
}

function runtimeTripleForArch(value) {
	switch (value) {
		case 'x64':
		case 'x86_64':
			return 'x86_64-pc-windows-msvc';
		default:
			console.error(`Unsupported Windows architecture: ${value}.`);
			process.exit(1);
	}
}

function packageArchForRuntimeTriple(runtimeTriple) {
	return runtimeTriple.split('-')[0];
}

async function copyRequired(source, destination) {
	try {
		await fsp.copyFile(source, destination);
	} catch (error) {
		throw new Error(`Missing required package file: ${source}`, { cause: error });
	}
}

function compressArchive(packageDir, archivePath) {
	const command = [
		'Compress-Archive',
		'-LiteralPath',
		quotePowerShellLiteral(packageDir),
		'-DestinationPath',
		quotePowerShellLiteral(archivePath),
		'-Force'
	].join(' ');

	run('powershell.exe', [
		'-NoProfile',
		'-ExecutionPolicy',
		'Bypass',
		'-Command',
		command
	]);
}

function quotePowerShellLiteral(value) {
	return `'${String(value).replace(/'/g, "''")}'`;
}

function run(command, args) {
	const result = spawnSync(command, args, { stdio: 'inherit' });
	if (result.error) {
		throw new Error(`Failed to run ${command}: ${result.error.message}`);
	}
	if (result.status !== 0) {
		throw new Error(`${command} exited with status ${result.status}`);
	}
}

main().catch((error) => {
	console.error('Failed to create Windows package.');
	console.error(error.message);
	process.exit(1);
});
