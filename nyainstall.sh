#!/bin/sh

if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: Cargo not found"
    echo "Use this command to install the Rust setup:"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && source \"\$HOME/.cargo/env\""
    rm -- "$0"
    exit 1
fi

if ! command -v git >/dev/null 2>&1; then
    echo "Error: git not found"
    echo "Install git with your system's package manager, then re-run this script"
    rm -- "$0"
    exit 1
fi

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1 && ! command -v clang >/dev/null 2>&1; then
    echo "Error: no C compiler found (cc/gcc/clang)"
    echo "Install a C compiler with your system's package manager, then re-run this script"
    rm -- "$0"
    exit 1
fi

owned_by_package_manager() {
    path="$1"
    if command -v dpkg >/dev/null 2>&1 && dpkg -S "$path" >/dev/null 2>&1; then
        return 0
    fi
    if command -v rpm >/dev/null 2>&1 && rpm -qf "$path" >/dev/null 2>&1; then
        return 0
    fi
    if command -v pacman >/dev/null 2>&1 && pacman -Qo "$path" >/dev/null 2>&1; then
        return 0
    fi
    if command -v brew >/dev/null 2>&1 && brew list --formula 2>/dev/null | grep -qx nyano; then
        return 0
    fi
    return 1
}

remove_foreign_install() {
    existing="$(command -v nyano 2>/dev/null)" || return 0
    if owned_by_package_manager "$existing"; then
        echo "Note: found nyano at $existing, managed by a package manager, leaving it alone"
        return 0
    fi
    echo "Removing existing nyano at $existing"
    if [ -w "$(dirname "$existing")" ]; then
        rm -f -- "$existing"
    elif sudo -n true 2>/dev/null; then
        sudo rm -f -- "$existing"
    elif which doas 2>/dev/null; then
        doas rm -f -- "$existing"
    else
        rm -f -- "$existing"
    fi
}

remove_foreign_install

# /usr/bin is protected by SIP on macOS; prefer /usr/local/bin, fall back to ~/.local/bin
if [ -w /usr/local/bin ]; then
    INSTALL_DIR="/usr/local/bin"
elif sudo -n true 2>/dev/null; then
    INSTALL_DIR="/usr/local/bin"
    USE_SUDO=1
elif which doas 2>/dev/null; then
    INSTALL_DIR="/usr/local/bin"
    USE_DOAS=1
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

INSTALL_PATH="$INSTALL_DIR/nyano"

rm -rf nyano-src
git clone https://github.com/shareui/nyano nyano-src
cd nyano-src
cargo build --release

echo "Installing to $INSTALL_PATH"
if [ -n "$USE_SUDO" ]; then
    sudo install -m 755 target/release/nyano "$INSTALL_PATH"
elif [ -n "$USE_DOAS" ]; then
    doas install -m 755 target/release/nyano "$INSTALL_PATH"
else
    install -m 755 target/release/nyano "$INSTALL_PATH"
fi

cd ..
rm -rf nyano-src

# When run through `curl | sh`, $0 is "sh" rather than a script path.
[ -f "$0" ] && rm -- "$0"

printf '\033[32mSuccessfully installed to %s!\033[0m\n' "$INSTALL_PATH"

# Remind the user if the install dir is not in PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) printf '\033[33mNote: add %s to your PATH:\n  export PATH="%s:$PATH"\033[0m\n' "$INSTALL_DIR" "$INSTALL_DIR" ;;
esac
