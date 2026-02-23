#!/bin/sh
set -eu

REPO="usealtoal/edgelord"
INSTALL_DIR="${EDGELORD_INSTALL_DIR:-$HOME/.edgelord/bin}"
NO_MODIFY_PATH="${EDGELORD_NO_MODIFY_PATH:-}"

main() {
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux) os="unknown-linux-musl" ;;
        darwin) os="apple-darwin" ;;
        *) echo "error: unsupported OS: $os" >&2; exit 1 ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "error: unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    target="${arch}-${os}"

    if [ -n "${EDGELORD_VERSION:-}" ]; then
        version="$EDGELORD_VERSION"
    else
        version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
            | grep '"tag_name"' | sed 's/.*"v\(.*\)".*/\1/')
    fi

    url="https://github.com/$REPO/releases/download/v${version}/edgelord-${target}.tar.gz"

    echo "downloading edgelord v${version} for ${target}..."
    tmpdir=$(mktemp -d)
    curl -fsSL "$url" | tar xz -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/edgelord" "$INSTALL_DIR/edgelord"
    chmod +x "$INSTALL_DIR/edgelord"
    rm -rf "$tmpdir"

    echo "installed edgelord to $INSTALL_DIR/edgelord"

    # Add to PATH if not already present
    if [ -z "$NO_MODIFY_PATH" ]; then
        case ":$PATH:" in
            *":$INSTALL_DIR:"*) ;;
            *)
                shell_config=""
                case "${SHELL:-}" in
                    */zsh) shell_config="$HOME/.zshrc" ;;
                    */bash)
                        if [ -f "$HOME/.bashrc" ]; then
                            shell_config="$HOME/.bashrc"
                        else
                            shell_config="$HOME/.bash_profile"
                        fi
                        ;;
                    */fish) shell_config="$HOME/.config/fish/config.fish" ;;
                esac

                if [ -n "$shell_config" ]; then
                    if ! grep -q "/.edgelord/bin" "$shell_config" 2>/dev/null; then
                        mkdir -p "$(dirname "$shell_config")"
                        case "${SHELL:-}" in
                            */fish)
                                printf '\n# Added by edgelord installer\nfish_add_path "$HOME/.edgelord/bin"\n' >> "$shell_config"
                                ;;
                            *)
                                printf '\n# Added by edgelord installer\nexport PATH="$HOME/.edgelord/bin:$PATH"\n' >> "$shell_config"
                                ;;
                        esac
                        echo "added $INSTALL_DIR to PATH in $shell_config"
                    fi
                    export PATH="$INSTALL_DIR:$PATH"
                else
                    echo "add $INSTALL_DIR to your PATH"
                fi
                ;;
        esac
    fi

    echo "run 'edgelord init' to get started"
}

main
