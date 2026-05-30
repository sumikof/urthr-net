FROM ubuntu:24.04

ARG USERNAME=ubuntu

# 1. 依存関係のインストールとクリーンアップ (軽量化・重複排除)
# --no-install-recommends で不要な推奨パッケージを省き、キャッシュを削除します
RUN apt-get update -y && apt-get install -y --no-install-recommends \
    curl git build-essential libssl-dev libudev-dev pkg-config \
    zlib1g-dev llvm clang cmake make libprotobuf-dev protobuf-compiler libclang-dev \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# 2. Node.js と Yarn/pnpm のインストール (Corepack の利用)
# corepack enable は USER 切替前=root 実行のため /usr/bin へシムを書ける。
# pnpm は web monorepo / Anchor.toml(package_manager = "pnpm")の前提。
# web/package.json の packageManager と同一バージョンをイメージに固定する。
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/* \
 && corepack enable yarn pnpm \
 && corepack prepare pnpm@9.10.0 --activate

# 3. 作業ディレクトリの作成と権限付与 (権限エラーの回避)
RUN mkdir -p /build && chown ${USERNAME}:${USERNAME} /build

USER ${USERNAME}
WORKDIR /build

# 4. Rustのインストール
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/${USERNAME}/.cargo/bin:$PATH"

# 5. Solana (Agave) のビルドとインストール
# エラー修正: 誤った cargo install をやめ、バイナリ抽出方式に変更
# 軽量化: ビルド完了後にソースコードとCargoのキャッシュを全削除
RUN curl -L https://github.com/anza-xyz/agave/archive/refs/tags/v3.1.14.tar.gz | tar -xzf - \
 && cd agave-3.1.14 \
 && cargo build --release \
 && mkdir -p /home/${USERNAME}/.local/bin /home/${USERNAME}/.local/lib \
 && find target/release -maxdepth 1 -type f -executable -exec cp {} /home/${USERNAME}/.local/bin/ \; \
 && find target/release -maxdepth 1 -type f -name "*.so" -exec cp {} /home/${USERNAME}/.local/lib/ \; \
 # Preserve the SBF SDK scaffold. cargo-build-sbf resolves the SDK relative to
 # its own location (<bin>/platform-tools-sdk/sbf); without this directory an
 # SBF/Anchor build fails with "Solana SDK path does not exist" in any fresh
 # environment. Pre-install platform-tools so the first build needs no network.
 && cp -r platform-tools-sdk /home/${USERNAME}/.local/bin/platform-tools-sdk \
 && /home/${USERNAME}/.local/bin/cargo-build-sbf --install-only \
 && cd /build \
 && rm -rf agave-3.1.14 \
 && rm -rf /home/${USERNAME}/.cargo/registry /home/${USERNAME}/.cargo/git

# Agaveの共有ライブラリ(.so)を認識させるためにパスを設定
ENV PATH="/home/${USERNAME}/.local/bin:$PATH"
ENV LD_LIBRARY_PATH="/home/${USERNAME}/.local/lib:$LD_LIBRARY_PATH"

# 6. Anchor-cli と Surfpool のビルドとインストール
# エラー修正: anchor-cli に --locked を付与
# 軽量化: ビルド後にCargoキャッシュを全削除
RUN cargo install --git https://github.com/solana-foundation/anchor --tag v1.0.2 anchor-cli --locked \
 && curl -L https://github.com/solana-foundation/surfpool/archive/refs/tags/v1.2.1.tar.gz | tar -xzf - \
 && cd surfpool-1.2.1 \
 && cargo surfpool-install \
 && cd /build \
 && rm -rf surfpool-1.2.1 \
 && rm -rf /home/${USERNAME}/.cargo/registry /home/${USERNAME}/.cargo/git

# 7. Corepack の pnpm バージョンを実行時ユーザーで固定
# corepack の activate 情報はユーザー別キャッシュ(~/.cache/node/corepack)に
# 保存される。line 21 の activate は root 実行のため、実行時ユーザー(ubuntu)
# からは見えず、corepack 同梱のデフォルト pnpm にフォールバックして
# web/package.json の packageManager(pnpm@9.10.0) と不一致になる。
# 実行時ユーザーで再度 activate し、9.10.0 を固定する。
RUN corepack prepare pnpm@9.10.0 --activate

# 8. ボリュームマウント先のディレクトリを ubuntu 所有で事前作成
# devcontainer.json で named volume にしている 4 つのパスは、Dockerfile 完了時点で
# 存在しない or 空になっている(.cargo/registry は line 50/65 で rm -rf 済み、
# .config/solana / .avm / .claude は image 内で一度も作成されない)。
# Docker は named volume を空ディレクトリへ初回マウントすると root:root の空ボリュームを
# 作るため、ubuntu ユーザでの書き込みが Permission denied になる
# (anchor build の cargo metadata、solana-keygen new、etc.)。
# 解決: image 内に ubuntu 所有の空ディレクトリを用意しておくと、Docker は初回マウント時に
# その所有/権限をボリュームへコピーする。
RUN mkdir -p \
    /home/${USERNAME}/.cargo/registry \
    /home/${USERNAME}/.config/solana \
    /home/${USERNAME}/.avm \
    /home/${USERNAME}/.claude
