# shared helpers for build_linux.py and build_macos.py

import hashlib
import re
import subprocess
import sys
import tarfile
from pathlib import Path

repoRoot = Path(__file__).resolve().parent.parent
cargoToml = repoRoot / "Cargo.toml"
templatesDir = repoRoot / "templates"

# archive extension produced for each package manager
managerExt = {
    "pacman": "pkg.tar.zst",
    "dnf": "rpm",
    "apt": "deb",
    "apk": "apk",
    "xbps": "xbps",
    "brew": "tar.gz",
}

# recipe file(s) copied out of templates/<manager>/ for each package manager
managerRecipeFiles = {
    "pacman": ["PKGBUILD"],
    "dnf": ["nyano.spec"],
    "apt": ["control", "copyright"],
    "apk": ["APKBUILD"],
    "xbps": ["template"],
    "brew": ["nyano.rb"],
}

# checksum algorithm each manager's recipe format expects
managerChecksumAlgo = {
    "pacman": "sha256",
    "dnf": "sha256",
    "apt": "sha256",
    "apk": "sha512",
    "xbps": "sha256",
    "brew": "sha256",
}

# managers whose recipe file is written once per architecture (the recipe
# itself is arch-specific, e.g. debian/control's Architecture field)
perArchManagers = {"apt", "apk", "xbps"}

# managers whose single recipe file covers both architectures at once via
# arch-suffixed placeholders, e.g. PKGBUILD's sha256sums_x86_64/_aarch64
combinedArchManagers = {"pacman", "brew"}


def readVersion():
    text = cargoToml.read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', text)
    if match is None:
        print("error: could not find version in Cargo.toml", file=sys.stderr)
        sys.exit(1)
    return match.group(1)


def buildBinary(triple):
    # reuses an already-built binary for this target if present, otherwise
    # cross-builds it via cargo; returns the binary path
    binaryPath = repoRoot / "target" / triple / "release" / "nyano"
    if binaryPath.exists():
        print(f"using existing binary for {triple}")
        return binaryPath

    cmd = ["cargo", "build", "--release", "--target", triple]
    result = subprocess.run(cmd, cwd=repoRoot)
    if result.returncode != 0:
        print(f"error: cargo build failed for target {triple}", file=sys.stderr)
        sys.exit(1)
    if not binaryPath.exists():
        print(f"error: expected binary not found at {binaryPath}", file=sys.stderr)
        sys.exit(1)
    return binaryPath


def hashOf(path, algo):
    hasher = hashlib.new(algo)
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def writeTarGz(binaryPath, outPath):
    with tarfile.open(outPath, "w:gz") as tar:
        tar.add(binaryPath, arcname="nyano")


def packageArchive(binaryPath, os_, arch, manager, outDir):
    # wraps the binary into the archive named for the given manager
    # note: this is the payload archive consumed by templates/<manager>/,
    # not a fully built native package (pacman/rpm/deb/apk/xbps binary
    # formats require their own host tooling to finish the packaging step)
    ext = managerExt[manager]
    outPath = outDir / f"nyano-{os_}-{arch}.{ext}"
    writeTarGz(binaryPath, outPath)
    algo = managerChecksumAlgo[manager]
    checksum = hashOf(outPath, algo)
    return outPath, algo, checksum


def checksumPlaceholder(algo, arch=None):
    base = "AUTOLOAD_SHA256" if algo == "sha256" else "AUTOLOAD_SHA512"
    if arch is None:
        return base
    return f"{base}_{arch.upper()}"


def writeRecipeSingle(manager, version, checksum, algo, destDir):
    # writes a manager's recipe once, per architecture, into its own subdir
    srcDir = templatesDir / manager
    destDir.mkdir(parents=True, exist_ok=True)
    placeholder = checksumPlaceholder(algo)

    for fileName in managerRecipeFiles[manager]:
        text = (srcDir / fileName).read_text(encoding="utf-8")
        text = text.replace("AUTOLOAD_VERSION", version)
        text = text.replace(placeholder, checksum)
        (destDir / fileName).write_text(text, encoding="utf-8")


def writeRecipeCombined(manager, version, checksumsByArch, destDir):
    # writes a manager's single recipe covering both architectures at once,
    # substituting each arch-suffixed checksum placeholder in turn
    srcDir = templatesDir / manager
    destDir.mkdir(parents=True, exist_ok=True)
    algo = managerChecksumAlgo[manager]

    for fileName in managerRecipeFiles[manager]:
        text = (srcDir / fileName).read_text(encoding="utf-8")
        text = text.replace("AUTOLOAD_VERSION", version)
        for arch, checksum in checksumsByArch.items():
            text = text.replace(checksumPlaceholder(algo, arch), checksum)
        (destDir / fileName).write_text(text, encoding="utf-8")


def writeChecksums(buildDir, checksums):
    checksumFile = buildDir / "SHA256SUMS"
    with open(checksumFile, "w", encoding="utf-8") as f:
        for name, algo, digest in sorted(checksums):
            f.write(f"{digest}  ({algo}) {name}\n")


def runTargets(targets, version, buildDir):
    # shared build loop: builds each target's binary, packages it for its
    # managers, and writes filled-in recipes; used identically by both
    # build_linux.py and build_macos.py with their own target lists
    recipesDir = buildDir / "recipes"
    checksums = []
    combinedArchChecksums = {manager: {} for manager in combinedArchManagers}

    for target in targets:
        binaryPath = buildBinary(target["triple"])
        for manager in target["managers"]:
            outPath, algo, checksum = packageArchive(binaryPath, target["os"], target["arch"], manager, buildDir)
            checksums.append((outPath.name, algo, checksum))
            print(f"packaged {outPath.relative_to(repoRoot)}")

            if manager in perArchManagers:
                writeRecipeSingle(manager, version, checksum, algo, recipesDir / manager / target["arch"])
            elif manager in combinedArchManagers:
                combinedArchChecksums[manager][target["arch"]] = checksum
            else:
                writeRecipeSingle(manager, version, checksum, algo, recipesDir / manager)

    for manager, checksumsByArch in combinedArchChecksums.items():
        if checksumsByArch:
            writeRecipeCombined(manager, version, checksumsByArch, recipesDir / manager)

    return checksums
