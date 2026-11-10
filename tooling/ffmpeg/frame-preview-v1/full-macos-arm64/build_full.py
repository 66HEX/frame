#!/usr/bin/env python3
"""Build the locked complete macOS ARM64 runtime in a new, private directory."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tarfile
import time
import urllib.request

HERE = Path(__file__).resolve().parent
LOCK_PATH = HERE / 'sources.lock.json'
LOCK = json.loads(LOCK_PATH.read_text())
ORDER = ['automake', 'meson', 'zlib', 'openssl', 'libxml2', 'graphite2',
         'fribidi', 'freetype', 'fontconfig', 'harfbuzz', 'libass', 'snappy',
         'srt', 'libvmaf', 'libklvanc', 'zimg', 'zvbi', 'aom', 'dav1d',
         'openjpeg', 'svt-av1', 'vvenc', 'libwebp', 'lame', 'opus', 'libogg',
         'libvorbis', 'libtheora', 'openh264', 'vpx', 'x264', 'x265', 'cargo-c', 'rav1e']
TOOLS = ['cmake', 'ninja', 'nasm', 'pkg-config', 'autoconf', 'autom4te',
         'autoheader', 'autoreconf', 'autoscan', 'autoupdate', 'ifnames',
         'glibtoolize', 'glibtool', 'rustup', 'cargo', 'rustc', 'rustdoc']


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def archive_name(name):
    # Archive format is autodetected; these names also match the verified cache.
    if name == 'cargo-c':
        return 'cargo-c.crate'
    url = LOCK['sources'][name]['url']
    suffix = '.tar.xz' if '.tar.xz' in url else '.tar.bz2' if '.tar.bz2' in url else '.tar.gz'
    return name + suffix


class Builder:
    def __init__(self, options):
        self.options = options
        self.root = options.output.absolute()
        if any(c.isspace() for c in str(self.root)):
            raise ValueError('Choose an output path without whitespace (upstream pkg-config limitation).')
        if platform.system() != 'Darwin' or platform.machine() not in ('arm64', 'aarch64'):
            raise ValueError('This recipe requires a macOS ARM64 host.')
        if sys.version_info < (3, 12):
            raise ValueError('Python 3.12+ is required for safe archive extraction.')
        # Cargo also reads ancestor configuration, independently of CARGO_HOME.
        # Keep dependency builds outside workspaces or ancestor Cargo configuration.
        ancestors = set(self.root.parents) | set(self.root.resolve().parents)
        for parent in ancestors:
            for relative in ['Cargo.toml', '.cargo/config', '.cargo/config.toml']:
                if (parent / relative).exists():
                    raise ValueError('Output has an ancestor Cargo workspace/configuration: ' + str(parent / relative))
        self.recipe = options.runtime_recipe.resolve(strict=True)
        runtime_lock = json.loads((self.recipe.parent / 'lock.json').read_text())
        if runtime_lock['source'] != LOCK['runtimeSource'] or runtime_lock['patch']['sha256'] != LOCK['runtimePatchSHA256']:
            raise ValueError('Runtime source/patch lock differs from this full-profile lock.')
        host_tools = {name: shutil.which(name) for name in TOOLS}
        missing = [name for name, path in host_tools.items() if not path]
        if missing:
            raise ValueError('Missing build tools (this script does not install system tools): ' + ', '.join(missing))
        subprocess.run([host_tools['rustup'], 'which', '--toolchain', LOCK['rustToolchain'], 'rustc'],
                       check=True, stdout=subprocess.DEVNULL)
        self.root.mkdir(parents=True, exist_ok=False)
        self.prefix = self.root / 'prefix'
        self.tools = self.root / 'tools'
        self.bin = self.tools / 'bin'
        for part in ['archives', 'records', 'logs', 'disposable', 'prefix', 'tools/bin']:
            (self.root / part).mkdir(parents=True, exist_ok=True)
        self.floor = options.min_free_gib * 1024**3
        self.disk()
        for name, path in host_tools.items():
            (self.bin / name).symlink_to(path)
        (self.bin / 'python3').symlink_to(sys.executable)
        (self.bin / 'libtoolize').symlink_to(host_tools['glibtoolize'])
        macros = Path(host_tools['glibtoolize']).resolve().parent.parent / 'share/aclocal'
        dest = self.tools / 'share/aclocal'
        dest.mkdir(parents=True)
        macro_records = []
        for name in ['libtool.m4', 'ltoptions.m4', 'ltsugar.m4', 'ltversion.m4', 'lt~obsolete.m4']:
            source = macros / name
            shutil.copy2(source, dest / name)
            macro_records.append({'path': str(source), 'sha256': sha(source)})
        # Do not inherit build/install/compiler overrides from the invoking shell.
        # In particular, DESTDIR escapes the private prefix and encoded Cargo flags
        # override RUSTFLAGS. Only host identity, locale, temp and HTTPS transport remain.
        allowed = {'HOME', 'USER', 'LOGNAME', 'TMPDIR', 'TMP', 'TEMP', 'LANG', 'LC_ALL',
                   'LC_CTYPE', 'TZ', 'DEVELOPER_DIR', 'RUSTUP_HOME', 'SSL_CERT_FILE',
                   'SSL_CERT_DIR', 'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'NO_PROXY',
                   'http_proxy', 'https_proxy', 'all_proxy', 'no_proxy'}
        self.env = {key: value for key, value in os.environ.items() if key in allowed}
        sdk = subprocess.check_output(['/usr/bin/xcrun', '--sdk', 'macosx', '--show-sdk-path'],
                                      env=self.env, text=True).strip()
        self.env.update(SDKROOT=sdk, CONFIG_SITE='/dev/null')
        flags = '-O2 -arch arm64 -mmacosx-version-min=12.0'
        self.env.update(PATH=str(self.bin) + ':/usr/bin:/bin:/usr/sbin:/sbin',
                        CC='/usr/bin/clang', CXX='/usr/bin/clang++',
                        MACOSX_DEPLOYMENT_TARGET='12.0', CFLAGS=flags, CXXFLAGS=flags,
                        CPPFLAGS='-I' + str(self.prefix / 'include'),
                        LDFLAGS='-L' + str(self.prefix / 'lib') + ' -arch arm64 -mmacosx-version-min=12.0',
                        PKG_CONFIG_LIBDIR=str(self.prefix / 'lib/pkgconfig') + ':' + str(self.prefix / 'share/pkgconfig'),
                        PKG_CONFIG_PATH='', CMAKE_PREFIX_PATH=str(self.prefix),
                        CMAKE_BUILD_PARALLEL_LEVEL=str(options.jobs), MAKEFLAGS='-j' + str(options.jobs) + ' V=1 VERBOSE=1',
                        ACLOCAL_PATH=str(dest), RUSTUP_AUTO_INSTALL='0')
        self.record('build-manifest', {'schema': 1, 'recipeSHA256': sha(__file__),
                    'lockSHA256': sha(LOCK_PATH), 'runtimeRecipeSHA256': sha(self.recipe),
                    'platform': platform.platform(), 'jobs': options.jobs, 'diskFloorBytes': self.floor,
                    'hostTools': host_tools, 'libtoolMacros': macro_records, 'sdkRoot': sdk})
        self.run('host', 'compiler', ['/usr/bin/clang', '--version'], self.root)
        self.run('host', 'sdk', ['/usr/bin/xcrun', '--show-sdk-path'], self.root)
        self.run('host', 'rust', [host_tools['rustup'], 'run', LOCK['rustToolchain'], 'rustc', '-vV'], self.root)

    def record(self, name, value):
        (self.root / 'records' / (name + '.json')).write_text(json.dumps(value, indent=2) + '\n')

    def disk(self):
        free = shutil.disk_usage(self.root if self.root.exists() else self.root.parent).free
        if free < self.floor:
            raise RuntimeError('Free disk is below the required reserve: ' + str(free))
        return free

    def run(self, name, phase, argv, cwd, extra=None):
        self.disk()
        env = self.env.copy()
        env.update(extra or {})
        argv = [str(x) for x in argv]
        stamp = str(time.time_ns())
        stem = name + '-' + phase + '-' + stamp
        log = self.root / 'logs' / (stem + '.log')
        start = time.monotonic()
        record = {'name': name, 'phase': phase, 'argv': argv, 'cwd': str(cwd),
                  'environment': {k: env[k] for k in ['PATH', 'CC', 'CXX', 'MACOSX_DEPLOYMENT_TARGET',
                   'CFLAGS', 'CXXFLAGS', 'CPPFLAGS', 'LDFLAGS', 'PKG_CONFIG_LIBDIR', 'PKG_CONFIG_PATH',
                   'CMAKE_PREFIX_PATH', 'CMAKE_BUILD_PARALLEL_LEVEL', 'MAKEFLAGS', 'ACLOCAL_PATH', 'SDKROOT', 'CONFIG_SITE']},
                  'extraEnvironment': extra or {}, 'freeBefore': self.disk(), 'log': str(log)}
        self.record(stem, record)
        with log.open('wb') as output:
            child = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                     stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while child.poll() is None:
                    time.sleep(2)
                    self.disk()
            except BaseException as error:
                # Stop every owned compiler immediately, before reaping the leader.
                # Waiting for a TERM-exiting leader first can orphan a TERM-ignoring worker.
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                child.wait(timeout=10)
                record['interrupted'] = repr(error)
                record['groupStopSignal'] = 'SIGKILL'
                raise
            finally:
                record.update(exit=child.returncode, seconds=time.monotonic() - start,
                              freeAfter=shutil.disk_usage(self.root).free)
                self.record(stem, record)
        print(name, phase, child.returncode, round(record['seconds'], 1), 's', flush=True)
        if child.returncode:
            raise RuntimeError('Build step failed; preserved log: ' + str(log))

    def fetch(self, name, pin=None):
        pin = pin or LOCK['sources'][name]
        filename = archive_name(name) if name in LOCK['sources'] else 'ffmpeg-n8.1.2.tar.gz'
        dest = self.root / 'archives' / filename
        cached = self.options.archive_cache / filename if self.options.archive_cache else None
        self.disk()
        if cached and cached.is_file():
            if sha(cached) != pin['sha256']:
                raise ValueError('Cached archive hash mismatch: ' + str(cached))
            shutil.copy2(cached, dest)
        else:
            partial = dest.with_suffix(dest.suffix + '.part')
            request = urllib.request.Request(pin['url'], headers={'User-Agent': 'Frame pinned runtime build'})
            with urllib.request.urlopen(request, timeout=60) as source, partial.open('xb') as output:
                size = 0
                while block := source.read(1024 * 1024):
                    size += len(block)
                    if size > 300 * 1024**2:
                        raise ValueError('Source archive exceeds 300 MiB bound')
                    self.disk()
                    output.write(block)
            if sha(partial) != pin['sha256']:
                raise ValueError('Downloaded archive hash mismatch: ' + name)
            partial.rename(dest)
        self.record('source-' + name, {'name': name, **pin, 'archive': str(dest), 'verifiedSHA256': sha(dest)})
        return dest

    def unpack(self, name):
        archive = self.fetch(name)
        root = self.root / 'disposable' / name
        root.mkdir()
        with tarfile.open(archive) as contents:
            contents.extractall(root, filter='data')
        entries = list(root.iterdir())
        return root, entries[0] if len(entries) == 1 and entries[0].is_dir() else root

    def finish(self, name, root):
        # Keep build configuration and licenses before deleting only this new intermediate tree.
        saved = self.root / 'records' / ('metadata-' + name)
        saved.mkdir()
        names = {'config.log', 'config.status', 'CMakeCache.txt', 'build.ninja',
                 'compile_commands.json', 'intro-buildoptions.json', 'Cargo.lock'}
        for item in root.rglob('*'):
            if item.is_file() and not item.is_symlink() and (item.name in names or item.name.upper().startswith(('LICENSE', 'COPYING'))):
                if item.stat().st_size < 20 * 1024**2:
                    target = saved / item.relative_to(root)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(item, target)
        files = [{'path': str(p.relative_to(self.prefix)), 'bytes': p.stat().st_size, 'sha256': sha(p)}
                 for p in self.prefix.rglob('*') if p.is_file() and p.suffix in ('.a', '.pc', '.h', '.hpp')]
        self.record('installed-after-' + name, {'files': files, 'removedIntermediate': str(root)})
        if root.parent != self.root / 'disposable':
            raise ValueError('Refusing to remove an unowned directory')
        shutil.rmtree(root)
        self.disk()

    def auto(self, name, source, options=(), pre=None, extra=None):
        if pre:
            self.run(name, 'autogen', pre, source, extra)
        self.run(name, 'configure', ['./configure', '--prefix=' + str(self.prefix), '--enable-static', '--disable-shared', *options], source, extra)
        make = ['make', '-j' + str(self.options.jobs)]
        if name == 'libvorbis':
            flags = re.search(r'^CFLAGS = (.*)$', (source / 'Makefile').read_text(), re.M).group(1)
            compatible = flags.replace('-force_cpusubtype_ALL', '').strip()
            self.record('vorbis-darwin-link-compatibility', {'configured': flags, 'used': compatible})
            make.append('CFLAGS=' + compatible)
        self.run(name, 'make', make, source, extra)
        self.run(name, 'install', ['make', 'install', *make[2:]], source, extra)

    def cmake_base(self):
        return ['cmake', '-G', 'Ninja', '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_INSTALL_PREFIX=' + str(self.prefix),
                '-DCMAKE_OSX_ARCHITECTURES=arm64', '-DCMAKE_OSX_DEPLOYMENT_TARGET=12.0', '-DCMAKE_POLICY_VERSION_MINIMUM=3.5',
                '-DCMAKE_IGNORE_PREFIX_PATH=/opt/homebrew;/usr/local', '-DCMAKE_PREFIX_PATH=' + str(self.prefix),
                '-DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF', '-DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF',
                '-DCMAKE_FIND_USE_CMAKE_ENVIRONMENT_PATH=OFF', '-DBUILD_SHARED_LIBS=OFF',
                '-DCMAKE_C_COMPILER=/usr/bin/clang', '-DCMAKE_CXX_COMPILER=/usr/bin/clang++', '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON']

    def cmake(self, name, source, options=()):
        build = source.parent / '_build'
        self.run(name, 'configure', [*self.cmake_base(), '-S', source, '-B', build, *options], source)
        self.run(name, 'make', ['cmake', '--build', build, '--parallel', str(self.options.jobs), '--verbose'], source)
        self.run(name, 'install', ['cmake', '--install', build], source)

    def meson(self, name, source, options=()):
        build = source.parent / '_build'
        self.run(name, 'configure', ['meson', 'setup', build, source, '--prefix', self.prefix, '--libdir=lib',
                 '--buildtype=release', '--default-library=static', *options], source)
        self.run(name, 'make', ['ninja', '-v', '-C', build, '-j' + str(self.options.jobs)], source)
        self.run(name, 'install', ['ninja', '-C', build, 'install'], source)

    def build(self, name):
        root, source = self.unpack(name)
        if name == 'automake':
            self.run(name, 'configure', ['./configure', '--prefix=' + str(self.tools)], source)
            self.run(name, 'make', ['make', '-j' + str(self.options.jobs)], source)
            self.run(name, 'install', ['make', 'install'], source)
        elif name == 'meson':
            target = self.tools / 'meson-source'
            shutil.move(source, target)
            script = self.bin / 'meson'
            script.write_text('#!/bin/sh\nexec ' + shlex.quote(sys.executable) + ' ' + shlex.quote(str(target / 'meson.py')) + ' "$@"\n')
            script.chmod(0o755)
            self.run(name, 'version', ['meson', '--version'], self.tools)
        elif name == 'zlib':
            self.run(name, 'configure', ['./configure', '--prefix=' + str(self.prefix), '--static'], source)
            self.run(name, 'make', ['make', '-j' + str(self.options.jobs)], source)
            self.run(name, 'install', ['make', 'install'], source)
        elif name == 'openssl':
            self.run(name, 'configure', ['./Configure', 'darwin64-arm64-cc', 'no-shared', 'no-tests',
                     '--prefix=' + str(self.prefix), '--libdir=lib', '--openssldir=' + str(self.prefix / 'ssl')], source)
            self.run(name, 'make', ['make', '-j' + str(self.options.jobs)], source)
            self.run(name, 'install', ['make', 'install_sw'], source)
        elif name in ('cargo-c', 'rav1e'):
            extra = {'CARGO_HOME': str(self.root / 'cargo-home'), 'CARGO_TARGET_DIR': str(root / 'cargo-target'),
                     'RUSTUP_TOOLCHAIN': LOCK['rustToolchain'], 'CARGO_BUILD_JOBS': str(self.options.jobs),
                     'CARGO_PROFILE_RELEASE_DEBUG': '0', 'RUSTFLAGS': '-C target-cpu=generic -C link-arg=-mmacosx-version-min=12.0'}
            if name == 'cargo-c':
                command = ['cargo', 'install', '--path', source, '--locked', '--root', self.tools,
                           '--target-dir', root / 'cargo-target', '--jobs', str(self.options.jobs), '--verbose']
            else:
                # Fetch with the installed Cargo; cargo-c consumes the same lock
                # and private cache without a second registry HTTP implementation.
                self.run(name, 'fetch', ['cargo', 'fetch', '--manifest-path', source / 'Cargo.toml',
                         '--locked', '--verbose'], source, extra)
                command = ['cargo', 'cinstall', '--manifest-path', source / 'Cargo.toml', '--locked', '--offline', '--release',
                           '--library-type', 'staticlib', '--prefix', self.prefix, '--libdir', self.prefix / 'lib',
                           '--target-dir', root / 'cargo-target', '--jobs', str(self.options.jobs), '--verbose']
            self.run(name, 'install', command, source, extra)
        elif name == 'openh264':
            options = ['ARCH=arm64', 'OS=darwin', 'PREFIX=' + str(self.prefix), 'STATIC_LDFLAGS=-lc++', 'V=Yes']
            self.run(name, 'make', ['make', '-j' + str(self.options.jobs), 'libraries', *options], source)
            self.run(name, 'install', ['make', 'install-static', *options], source)
        elif name in ('vpx', 'x264'):
            options = ['--target=arm64-darwin21-gcc', '--disable-unit-tests', '--disable-examples', '--disable-tools',
                       '--disable-docs', '--enable-vp9-highbitdepth', '--enable-static', '--disable-shared'] if name == 'vpx' else [
                       '--enable-static', '--disable-cli', '--enable-pic', '--bit-depth=all', '--chroma-format=all']
            self.run(name, 'configure', ['./configure', '--prefix=' + str(self.prefix), *options], source)
            self.run(name, 'make', ['make', '-j' + str(self.options.jobs)], source)
            self.run(name, 'install', ['make', 'install'], source)
        elif name == 'x265':
            base = [*self.cmake_base(), '-DENABLE_SHARED=OFF', '-DENABLE_CLI=OFF', '-DNATIVE_BUILD=OFF']
            for bits in (10, 12):
                build = root / ('build' + str(bits))
                options = ['-DHIGH_BIT_DEPTH=ON', '-DEXPORT_C_API=OFF'] + (['-DMAIN12=ON'] if bits == 12 else [])
                self.run(name, 'configure' + str(bits), [*base, '-S', source / 'source', '-B', build, *options], source)
                self.run(name, 'make' + str(bits), ['cmake', '--build', build, '--parallel', str(self.options.jobs), '--verbose'], source)
            build = root / 'build8'
            build.mkdir()
            for bits in (10, 12):
                (build / ('libx265_' + str(bits) + 'bit.a')).symlink_to(root / ('build' + str(bits)) / 'libx265.a')
            self.run(name, 'configure8', [*base, '-S', source / 'source', '-B', build, '-DEXTRA_LINK_FLAGS=-L' + str(build),
                     '-DEXTRA_LIB=x265_10bit.a;x265_12bit.a', '-DLINKED_10BIT=ON', '-DLINKED_12BIT=ON'], source)
            self.run(name, 'make8', ['cmake', '--build', build, '--parallel', str(self.options.jobs), '--verbose'], source)
            (build / 'libx265.a').rename(build / 'libx265_8bit.a')
            self.run(name, 'merge', ['/usr/bin/libtool', '-static', '-o', build / 'libx265.a',
                     build / 'libx265_8bit.a', build / 'libx265_10bit.a', build / 'libx265_12bit.a'], source)
            self.run(name, 'install', ['cmake', '--install', build], source)
        elif name in CMAKE_OPTIONS:
            self.cmake(name, source, CMAKE_OPTIONS[name] + (['-DOPENSSL_ROOT_DIR=' + str(self.prefix)] if name == 'srt' else []))
        elif name in MESON_OPTIONS:
            self.meson(name, source / 'libvmaf' if name == 'libvmaf' else source, MESON_OPTIONS[name])
        else:
            pre = ['./autogen.sh', '--build'] if name == 'libklvanc' else ['./autogen.sh'] if name == 'zimg' else None
            self.auto(name, source, AUTO_OPTIONS[name], pre, {'STL_LIBS': '-lc++'} if name == 'zimg' else None)
        self.finish(name, root)

    def runtime(self):
        source = LOCK['runtimeSource']
        archive = self.fetch('ffmpeg', {'version': source['version'], 'url': source['url'], 'sha256': source['archiveSHA256']})
        config = ['--prefix=' + str(self.root / 'runtime-install'), '--extra-version=frame-preview-v1', '--enable-static',
                  '--disable-shared', '--disable-debug', '--disable-doc', '--arch=aarch64', '--target-os=darwin',
                  '--cc=/usr/bin/clang', '--cxx=/usr/bin/clang++', '--pkg-config-flags=--static', '--extra-libs=-lc++',
                  '--extra-cflags=' + self.env['CFLAGS'] + ' -I' + str(self.prefix / 'include'),
                  '--extra-ldflags=' + self.env['LDFLAGS'], *LOCK['ffmpegFeatures']]
        path = self.root / 'configure-full.json'
        path.write_text(json.dumps(config, indent=2) + '\n')
        self.run('ffmpeg', 'build-pinned', [sys.executable, self.recipe, '--archive', archive, '--output', self.root / 'runtime',
                 '--configure-json', path, '--build', '--jobs', str(self.options.jobs)], self.root)


AUTO_OPTIONS = {
    'libxml2': ['--without-python', '--without-lzma'], 'fribidi': ['--disable-docs'],
    'freetype': ['--with-harfbuzz=no', '--with-png=no', '--with-brotli=no'],
    'fontconfig': ['--enable-libxml2', '--disable-docs', '--disable-nls'],
    'libass': ['--disable-require-system-font-provider'], 'libklvanc': [], 'zimg': [],
    'zvbi': ['--disable-dvb', '--disable-bktr', '--disable-proxy', '--without-x', '--disable-nls'],
    'lame': ['--disable-frontend'], 'opus': ['--disable-extra-programs', '--disable-doc'], 'libogg': [],
    'libvorbis': ['--disable-examples'],
    'libtheora': ['--disable-examples', '--disable-oggtest', '--disable-vorbistest', '--disable-sdltest', '--disable-doc']}
CMAKE_OPTIONS = {
    'graphite2': ['-DGRAPHITE2_COMPARE_RENDERER=OFF', '-DGRAPHITE2_NTRACING=ON', '-DBUILD_TESTING=OFF'],
    'snappy': ['-DSNAPPY_BUILD_BENCHMARKS=OFF', '-DSNAPPY_BUILD_TESTS=OFF'],
    'srt': ['-DENABLE_SHARED=OFF', '-DENABLE_APPS=OFF', '-DENABLE_TESTING=OFF', '-DUSE_ENCLIB=openssl'],
    'aom': ['-DENABLE_TESTS=OFF', '-DENABLE_EXAMPLES=OFF', '-DENABLE_TOOLS=OFF', '-DAOM_TARGET_CPU=arm64'],
    'openjpeg': ['-DBUILD_CODEC=OFF', '-DBUILD_TESTING=OFF'],
    'svt-av1': ['-DBUILD_APPS=OFF', '-DBUILD_TESTING=OFF', '-DENABLE_AVX512=OFF', '-DNATIVE=OFF'],
    'vvenc': ['-DVVENC_ENABLE_LINK_TIME_OPT=OFF', '-DVVENC_INSTALL_FULLFEATURE_APP=OFF', '-DVVENC_BUILD_TESTS=OFF'],
    'libwebp': ['-DWEBP_BUILD_ANIM_UTILS=OFF', '-DWEBP_BUILD_CWEBP=OFF', '-DWEBP_BUILD_DWEBP=OFF',
               '-DWEBP_BUILD_GIF2WEBP=OFF', '-DWEBP_BUILD_IMG2WEBP=OFF', '-DWEBP_BUILD_VWEBP=OFF',
               '-DWEBP_BUILD_WEBPINFO=OFF', '-DWEBP_BUILD_WEBPMUX=OFF', '-DWEBP_BUILD_EXTRAS=OFF']}
MESON_OPTIONS = {
    'harfbuzz': ['-Dglib=disabled', '-Dgobject=disabled', '-Dcairo=disabled', '-Dicu=disabled', '-Dgraphite2=enabled',
                 '-Dtests=disabled', '-Ddocs=disabled', '-Dutilities=disabled'],
    'libvmaf': ['-Denable_tests=false', '-Denable_docs=false', '-Denable_avx512=false'],
    'dav1d': ['-Denable_tools=false', '-Denable_tests=false']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--build', action='store_true', help='Download and build; otherwise print the locked plan only.')
    parser.add_argument('--archive-cache', type=Path, help='Read-only cache; each reused archive must match its lock hash.')
    parser.add_argument('--verify-cache', type=Path, help='Verify all 34 locked dependency/tool archives without building.')
    parser.add_argument('--runtime-recipe', type=Path, default=HERE.parent / 'build_pinned.py')
    parser.add_argument('--jobs', type=int, default=4)
    parser.add_argument('--min-free-gib', type=float, default=6)
    options = parser.parse_args()
    if not 1 <= options.jobs <= 4 or not math.isfinite(options.min_free_gib) or options.min_free_gib < 6:
        parser.error('jobs must be1..4 and disk reserve at least6GiB')
    if options.verify_cache:
        results = {name: sha(options.verify_cache / archive_name(name)) == pin['sha256']
                   for name, pin in LOCK['sources'].items()}
        print(json.dumps(results, indent=2))
        if not all(results.values()):
            raise SystemExit(1)
        return
    if not options.build:
        print(json.dumps({'buildOrder': ORDER, 'lock': LOCK, 'output': str(options.output),
                          'jobs': options.jobs, 'diskReserveGiB': options.min_free_gib}, indent=2))
        return
    if options.output is None:
        parser.error('--build requires a new --output directory')
    builder = Builder(options)
    for name in ORDER:
        builder.build(name)
    builder.runtime()
    print(json.dumps({'artifacts': str(builder.root / 'runtime/artifacts'), 'built': True}))


if __name__ == '__main__':
    main()
