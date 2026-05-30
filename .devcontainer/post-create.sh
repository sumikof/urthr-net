#!/usr/bin/env bash
set -euo pipefail

# Preflight: named volume が ubuntu 所有か検証
#
# devcontainer.json で named volume にしている 4 つのパスは、Dockerfile 末尾で
# ubuntu 所有の空ディレクトリとして事前作成している(Docker の初回マウント時
# copy-up に乗せるため)。しかし古いイメージで作られたボリュームが残っていると、
# それらは root:root で初期化された状態で再利用されてしまい、
# 後続の solana-keygen / anchor build が Permission denied で死ぬ。
#
# ここで早期に検出し、ホスト側の復旧手順を提示して exit する。
preflight_volume_ownership() {
  local me owner bad=()
  me="$(id -u)"
  for dir in \
    "$HOME/.cargo/registry" \
    "$HOME/.config/solana" \
    "$HOME/.avm" \
    "$HOME/.claude"
  do
    [ -d "$dir" ] || continue
    owner="$(stat -c %u "$dir")"
    [ "$owner" = "$me" ] || bad+=("$dir (owner uid=$owner, expected $me)")
  done
  if [ "${#bad[@]}" -gt 0 ]; then
    cat >&2 <<EOF
ERROR: 以下の named volume が ubuntu 所有ではないため書き込みできません:
$(printf '  - %s\n' "${bad[@]}")

これは古い devcontainer ビルドで作成された壊れたボリュームが残っているためです。
ホスト側のシェルで以下を実行してから devcontainer をリビルドしてください:

  # 壊れたボリュームを削除(走行中の devcontainer は事前に停止してください)
  docker volume ls --format '{{.Name}}' \\
    | grep -E '^(cargo-registry|solana-config|avm-cache|claude-code-config)-' \\
    | xargs -r docker volume rm

その後 "Dev Containers: Rebuild Container" を実行してください。
EOF
    exit 1
  fi
}

preflight_volume_ownership

solana config set --url localhost

if [ ! -f "$HOME/.config/solana/id.json" ]; then
  solana-keygen new --no-bip39-passphrase --silent --outfile "$HOME/.config/solana/id.json"
fi

echo "=== Installed versions ==="
rustc --version
solana --version
anchor --version
node --version
pnpm --version

# --ignore-scripts: skip optional native build scripts (bufferutil, utf-8-validate).
# pnpm v10+ exits non-zero on unbuilt scripts, which aborts this script under
# `set -e`. ws falls back to pure JS, which is fine for dev/test.
if [ -f "package.json" ]; then
  pnpm install --ignore-scripts
fi

if [ -f "web/package.json" ]; then
  pnpm -C web install --ignore-scripts
fi
