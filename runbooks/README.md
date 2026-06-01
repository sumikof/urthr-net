# workspace Runbook 集

[![Surfpool](https://img.shields.io/badge/Operated%20with-Surfpool-gree?labelColor=gray)](https://surfpool.run)

## 利用可能な Runbook

### deployment
プログラムのデプロイ

## はじめに

このリポジトリは開発ワークフローの一環として [Surfpool](https://surfpool.run) を使用しています。

Surfpool は Solana 開発体験に3つの主要な機能強化をもたらします。
- **Surfnet**: マシン上で動作するローカルバリデータ。オンザフライでメインネットをフォークできるため、プログラムをテストする際に常に最新のチェーンデータを使用できます。
- **Runbooks**: `infrastructure as code` という DevOps のベストプラクティスを Solana に導入します。オンチェーン操作とデプロイのための安全で再現可能、かつ構成可能なスクリプトを記述できます。
- **Surfpool Studio**: 完全ローカルの Web UI で、トランザクションに対する新しいレベルの内省機能を提供します。

### インストール

Surfpool インストーラ:

```console
curl -sL https://run.surfpool.run/ | bash
```

ソースからインストール:

```console
# リポジトリをクローン
git clone https://github.com/solana-foundation/surfpool.git

# リポジトリをカレントディレクトリに設定
cd surfpool

# ビルド
cargo surfpool-install
```

### Surfnet を起動する

```console
$ surfpool start
```

## リソース

Surfnet と Runbook 構文を理解し、surfpool の強力な機能を発見するには、[docs.surfpool.run](https://docs.surfpool.run) のチュートリアルとドキュメントにアクセスしてください。

また、[Visual Studio Code 拡張機能](https://marketplace.visualstudio.com/items?itemName=txtx.txtx) を使用すると Runbook の記述が容易になります。

[Surfpool 101 シリーズ](https://www.youtube.com/playlist?list=PL0FMgRjJMRzO1FdunpMS-aUS4GNkgyr3T) も Surfpool とその機能について学び始めるのに最適な場所です。
<a href="https://www.youtube.com/playlist?list=PL0FMgRjJMRzO1FdunpMS-aUS4GNkgyr3T">
  <picture>
    <source srcset="https://raw.githubusercontent.com/solana-foundation/surfpool/main/doc/assets/youtube.png">
    <img alt="Surfpool 101 series" style="max-width: 100%;">
  </picture>
</a>

## クイックスタート

### このリポジトリで利用可能な Runbook を一覧表示する
```console
$ surfpool ls
Name                                    Description
deployment                              Deploy programs
```

### Surfnet を起動し、プログラム再コンパイル時に `deployment` Runbook を自動実行する:
```console
$ surfpool start --watch
```

### 既存の Runbook を実行する
```console
$ surfpool run deployment
```
