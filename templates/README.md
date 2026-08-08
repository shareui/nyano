# Packaging templates

One directory per package manager, each holding the recipe file(s) that
package manager expects.

Two kinds of placeholders:

- `AUTOLOAD_*` — filled in automatically by `scripts/build.py` (version
  from Cargo.toml, sha256/sha512 checksum of the built archive). Never
  edit these by hand, they get overwritten on every build.
- `PLACE_*` — left for the packager to fill in by hand (changelog dates,
  changelog entries, debian Architecture field per build).

Project metadata (homepage, bug tracker, maintainer, license) is already
filled in below and does not need editing unless it changes:

- Homepage: https://github.com/shareui/nyano
- Bug reports: https://github.com/shareui/nyano/issues
- Maintainer: shareui <shareui@proton.me>
- License: MIT

Files:

- `pacman/PKGBUILD` — Arch Linux
- `dnf/nyano.spec` — Fedora / RHEL (RPM spec)
- `apt/control`, `apt/copyright` — Debian / Ubuntu
- `apk/APKBUILD` — Alpine Linux
- `xbps/template` — Void Linux
- `brew/nyano.rb` — Homebrew (macOS)

`scripts/build.py` compiles the project for each target, packages the
resulting binary, and writes a filled-in copy of each recipe into
`build/{version}/recipes/`.
