#!/bin/sh
set -eu

version=${KOTOBA_VERSION:-latest}
prefix=${KOTOBA_INSTALL_PREFIX:-"${XDG_DATA_HOME:-$HOME/.local/share}/kotoba"}
bindir=${KOTOBA_BIN_DIR:-"$HOME/.local/bin"}
case "$(uname -s):$(uname -m)" in
  Darwin:arm64) platform=darwin-arm64 ;;
  Darwin:x86_64) platform=darwin-amd64 ;;
  Linux:x86_64) platform=linux-amd64 ;;
  *) echo "kotoba installer: unsupported platform $(uname -s)/$(uname -m)" >&2; exit 1 ;;
esac
if [ "$version" = latest ]; then
  release_path=latest/download
else
  release_path=download/$version
fi
archive_url=${KOTOBA_ARCHIVE_URL:-"https://github.com/kotoba-lang/kotoba/releases/$release_path/kotoba-$platform.tar.gz"}
checksum_url=${KOTOBA_CHECKSUM_URL:-"$archive_url.sha256"}

command -v curl >/dev/null 2>&1 || { echo "kotoba installer: curl is required" >&2; exit 1; }
command -v tar >/dev/null 2>&1 || { echo "kotoba installer: tar is required" >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  echo "kotoba installer: sha256sum or shasum is required" >&2; exit 1
fi

tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/kotoba-install.XXXXXX")
trap 'rm -rf "$tmpdir"' EXIT HUP INT TERM

curl -fL "$archive_url" -o "$tmpdir/kotoba.tar.gz"
curl -fL "$checksum_url" -o "$tmpdir/kotoba.tar.gz.sha256"
expected=$(awk '{print $1}' "$tmpdir/kotoba.tar.gz.sha256")
[ -n "$expected" ] || { echo "kotoba installer: published checksum is empty" >&2; exit 1; }
actual=$(sha256 "$tmpdir/kotoba.tar.gz")
[ -n "$actual" ] || { echo "kotoba installer: could not compute checksum" >&2; exit 1; }
[ "$actual" = "$expected" ] || { echo "kotoba installer: checksum mismatch" >&2; exit 1; }
tar -xzf "$tmpdir/kotoba.tar.gz" -C "$tmpdir"
[ -x "$tmpdir/kotoba" ] || { echo "kotoba installer: native executable missing" >&2; exit 1; }

install_dir="$prefix/$version"
mkdir -p "$install_dir" "$bindir"
cp "$tmpdir/kotoba" "$install_dir/kotoba"
chmod +x "$install_dir/kotoba"
ln -sfn "$version" "$prefix/current"

cat >"$bindir/kotoba" <<EOF
#!/bin/sh
exec "$prefix/current/kotoba" "\$@"
EOF
chmod +x "$bindir/kotoba"

echo "kotoba $version installed at $install_dir"
echo "Add $bindir to PATH if kotoba is not found."
