#!/usr/bin/env python3
# cross-compiles nyano for the linux x86_64/aarch64 targets, packages the
# resulting binary for pacman, dnf, apt, apk and xbps, and fills in the
# AUTOLOAD_* placeholders (version, checksum) in each recipe template.
# writes everything into build/{version}/
# macos targets are handled separately by build_macos.py

from _common import repoRoot, readVersion, runTargets, writeChecksums

targets = [
    {"os": "linux", "arch": "x86_64", "triple": "x86_64-unknown-linux-gnu", "managers": ["pacman", "dnf", "apt", "apk", "xbps"]},
    {"os": "linux", "arch": "aarch64", "triple": "aarch64-unknown-linux-gnu", "managers": ["pacman", "dnf", "apt", "apk", "xbps"]},
]


def main():
    version = readVersion()
    buildDir = repoRoot / "build" / version
    buildDir.mkdir(parents=True, exist_ok=True)

    checksums = runTargets(targets, version, buildDir)

    writeChecksums(buildDir, checksums)
    print(f"done: version {version}, output in {buildDir.relative_to(repoRoot)}")


if __name__ == "__main__":
    main()
