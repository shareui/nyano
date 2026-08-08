#!/usr/bin/env python3
# cross-compiles nyano for the macos x86_64/aarch64 targets, packages the
# resulting binary for brew, and fills in the AUTOLOAD_* placeholders
# (version, checksum) in templates/brew/nyano.rb.
# writes everything into build/{version}/
# must be run on a macos host (or with osxcross set up); linux targets are
# handled separately by build_linux.py

from _common import repoRoot, readVersion, runTargets, writeChecksums

targets = [
    {"os": "macos", "arch": "x86_64", "triple": "x86_64-apple-darwin", "managers": ["brew"]},
    {"os": "macos", "arch": "aarch64", "triple": "aarch64-apple-darwin", "managers": ["brew"]},
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
