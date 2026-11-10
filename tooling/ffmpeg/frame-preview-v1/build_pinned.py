#!/usr/bin/env python3
"""Prepare/build an isolated pinned runtime. Never installs or replaces a runtime."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import shutil
import subprocess
import tarfile
import time

HERE = Path(__file__).resolve().parent

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--archive', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--configure-json', type=Path, default=HERE/'configure-minimal-macos-arm64.json')
    p.add_argument('--build', action='store_true', help='Compile after preparing; default only prepares and verifies.')
    p.add_argument('--jobs', type=int, default=4)
    args = p.parse_args()
    if not 1 <= args.jobs <= 64:
        p.error('--jobs must be 1..64')
    lock = json.loads((HERE/'lock.json').read_text())
    archive = args.archive.resolve(strict=True)
    patch = HERE/lock['patch']['path']
    if sha(archive) != lock['source']['archiveSHA256'] or sha(patch) != lock['patch']['sha256']:
        p.error('Pinned archive or patch SHA256 mismatch')
    config = json.loads(args.configure_json.read_text())
    if not isinstance(config, list) or not all(isinstance(x, str) and '\0' not in x for x in config):
        p.error('Configure JSON must contain an argv string array')
    dest = args.output.absolute()
    dest.mkdir(parents=True, exist_ok=False)
    source = dest/'source'; source.mkdir()
    build = dest/'build'; build.mkdir()
    record = {'schema':1, 'platform':platform.platform(), 'machine':platform.machine(),
              'archiveSHA256':sha(archive), 'patchSHA256':sha(patch),
              'recipeSHA256':sha(__file__), 'configureJSONSHA256':sha(args.configure_json),
              'configureArgs':config, 'buildRequested':args.build, 'steps':[]}
    def save():
        (dest/'build-record.json').write_text(json.dumps(record, indent=2)+'\n')
    def run(name, cmd, cwd):
        started = time.monotonic()
        with (dest/(name+'.log')).open('wb') as log:
            result = subprocess.run(cmd, cwd=cwd, stdin=subprocess.DEVNULL,
                                    stdout=log, stderr=subprocess.STDOUT, check=False)
        record['steps'].append({'name':name,'argv':cmd,'cwd':str(cwd),
                                'exit':result.returncode,'seconds':time.monotonic()-started})
        save()
        if result.returncode:
            raise SystemExit('Failed '+name+'; see preserved log')
    save()
    root = lock['source']['archiveRoot']
    with tarfile.open(archive, 'r:gz') as tar:
        for member in tar:
            name = PurePosixPath(member.name)
            if not name.parts or name.parts[0] != root or name.is_absolute() or '..' in name.parts:
                raise SystemExit('Unexpected archive path: '+member.name)
            relative = name.parts[1:]
            if not relative:
                continue
            target = source.joinpath(*relative)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                with tar.extractfile(member) as src, target.open('xb') as out:
                    shutil.copyfileobj(src, out)
                target.chmod(member.mode & 0o777)
            else:
                raise SystemExit('Unsupported archive entry: '+member.name)
    for entry in lock['files']:
        if sha(source/entry['path']) != entry['upstreamSHA256']:
            raise SystemExit('Upstream file hash mismatch: '+entry['path'])
    run('apply-patch', ['patch','--batch','--fuzz=0','-p1','-i',str(patch)], source)
    for entry in lock['files']:
        if sha(source/entry['path']) != entry['patchedSHA256']:
            raise SystemExit('Patched file hash mismatch: '+entry['path'])
    record['sourceVerified'] = True
    save()
    if args.build:
        run('configure', [str(source/'configure'),*config], build)
        run('make', ['make','-j'+str(args.jobs),'ffmpeg','ffprobe'], build)
        artifacts = dest/'artifacts'; artifacts.mkdir()
        for name in ('ffmpeg','ffprobe'):
            shutil.copy2(build/name, artifacts/name)
        record['binaries'] = {x:sha(artifacts/x) for x in ('ffmpeg','ffprobe')}
        save()
    print(json.dumps({'output':str(dest),'sourceVerified':True,'built':args.build}))

if __name__ == '__main__':
    main()
