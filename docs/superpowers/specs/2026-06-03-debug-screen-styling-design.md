# デバッグ画面のスタイル刷新 ＋ アドレスコピーボタン — 設計

- 日付: 2026-06-03
- 対象: `app/`（UrthrNet localnet デバッグハーネス）
- 範囲: 「見た目を軽く整える」＋ 後続で使う値（生成 mint アドレス・ウォレットアドレス）へのコピーボタン追加

## 背景と問題

`app/src/index.css` には既に整ったデザインシステム（ライト/ダーク両対応の CSS 変数 `--accent`/`--border`/`--code-bg`/`--shadow`/`--mono` 等）が定義済み。しかしデバッグ画面の各パネルはこれを一切使わず、インラインの
ハードコード色（`#ccc`/`#666`/`crimson`/`green`）で描画されている。さらに `#root` は `text-align: center` ＋幅 `1126px` 固定なのに `App.tsx` は `maxWidth: 880; margin: 2rem auto` を
インライン指定しており、中央寄せの文字組みとパネルの左揃えが噛み合っていない。

また、`CreateMintPanel` が生成する mint アドレスや接続ウォレットのアドレスは後続のパネル入力（mintTo / create_campaign 等）へ手で貼り付ける必要があるが、コピー導線が無い。

## ゴール / 非ゴール

**ゴール**
- デバッグ画面の見た目を `index.css` のデザイン変数に寄せて統一し、ホバー/フォーカス状態とダークモード対応を得る。
- 生成 mint アドレスと接続ウォレットアドレスにコピーボタンを付ける。

**非ゴール（YAGNI）**
- レイアウト再設計（ナビ/サイドバー/セクション折りたたみ/グリッド化）。
- 署名（signature）・インスペクタ JSON 内アドレスへのコピーボタン。
- 状態色（green/crimson）の変数化。
- 各命令パネルの個別手入れ。
- 共有プレゼンテーションコンポーネント（Card/Button/Field）の新規化（過剰）。

## 方針

既存 `index.css` のデザイン変数を唯一の真実とし、デバッグ画面のハードコード色／インライン style をそこへ寄せる。
ほとんどのパネルは共通コア（`DebugPanel` / `TextField` / `AccountInspector`）を経由して描画されるため、**このコア3つ＋localnet 2パネル＋AccountInfo を直すだけで画面のほぼ全面に効く**。
パネルの DOM 構造・ロジック・送信フローは変えない（＝送信前シミュレーション必須の不変条件はそのまま維持）。

採用案: **index.css に意味付けクラスを追加し、共通コアに `className` を当てる**（インライン style からクラスへ移行）。
不採用案: 共有 style モジュール（`:hover`/`:focus` 擬似状態が作れない）／共有コンポーネント新規化（変更量が「軽く」の範囲を超える）。

## 変更対象ファイル（8ファイル、うち新規1）

| ファイル | 変更内容 |
|---|---|
| `app/src/index.css` | デバッグ画面用クラス群を追記（変数ベース） |
| `app/src/debug/core/DebugPanel.tsx` | `box`／ボタン／結果表示のインライン style を `className` 化 |
| `app/src/debug/core/fields.tsx` | `TextField` を `className` 化 |
| `app/src/debug/accounts/AccountInspector.tsx` | `box`・結果 `pre` を `className` 化 |
| `app/src/components/CopyButton.tsx` | **新規** 共通コピーボタン |
| `app/src/debug/localnet/CreateMintPanel.tsx` | 生成 mint アドレス表示に `CopyButton` を追加 |
| `app/src/components/AccountInfo.tsx` | ウォレットアドレス表示に `CopyButton` を追加 |
| `app/src/App.tsx` | レイアウト整合と見出しの体裁を `className` 化 |

**触らないもの**: 各命令パネル（`InitializeProtocolPanel` 等）は共通コア経由のため個別変更なし。生成コード（`app/src/generated`）も対象外。

## CSS クラス設計（index.css への追記）

既存変数のみを使用しハードコード色を排除する。状態色（green/crimson）は可読性優先で固定色のまま据え置く。

```css
/* パネル枠（fieldset の box 置換） */
.panel { border: 1px solid var(--border); border-radius: 10px;
         padding: 16px 20px; margin: 16px 0; background: var(--bg);
         box-shadow: var(--shadow); text-align: left; }
.panel > legend { font-weight: 600; color: var(--text-h); padding: 0 6px; }

/* 入力フィールド（TextField） */
.field { display: flex; align-items: center; gap: 12px; margin: 8px 0; }
.field > .field-label { min-width: 180px; color: var(--text); }
.field > input { flex: 1; max-width: 380px; padding: 6px 10px;
                 border: 1px solid var(--border); border-radius: 6px;
                 background: var(--bg); color: var(--text-h); font: inherit; }
.field > input:focus { outline: none; border-color: var(--accent);
                       box-shadow: 0 0 0 3px var(--accent-bg); }
.field-error { color: crimson; font-size: 0.9em; }

/* ボタン（共通＋primary＋copy） */
.btn { padding: 6px 14px; border: 1px solid var(--border); border-radius: 6px;
       background: var(--bg); color: var(--text-h); font: inherit; cursor: pointer;
       transition: background .12s, border-color .12s; }
.btn:hover:not(:disabled) { border-color: var(--accent); background: var(--accent-bg); }
.btn:disabled { opacity: .5; cursor: not-allowed; }
.btn-primary { border-color: var(--accent-border); background: var(--accent-bg); color: var(--accent); }
.btn-primary:hover:not(:disabled) { background: var(--accent); color: #fff; }
.btn-copy { font-size: 0.8em; padding: 2px 8px; }
.panel-actions { display: flex; gap: 8px; margin-top: 12px; }

/* 結果表示（成功/失敗/ログ/署名/エラー） */
.result { margin-top: 12px; font-size: 0.9em; }
.result-ok { color: green; }      /* 成功・署名 */
.result-err { color: crimson; }   /* 失敗・エラー */
.result pre, .inspector-json { background: var(--code-bg); border-radius: 6px;
       padding: 10px; font: 0.8em var(--mono); white-space: pre-wrap; word-break: break-all; }
.addr { font: 0.9em var(--mono); background: var(--code-bg);
        padding: 2px 6px; border-radius: 4px; word-break: break-all; }

/* レイアウト */
.app { max-width: 920px; margin: 0 auto; padding: 24px 20px 64px; text-align: left; }
.app section { margin-top: 28px; }
.app section > h2 { border-bottom: 1px solid var(--border); padding-bottom: 6px; }
```

ボタン主従: 「シミュレート」= `.btn`、「承認して送信」= `.btn-primary` として視覚的主従を付ける。

## CopyButton コンポーネント

**新規 `app/src/components/CopyButton.tsx`**

```tsx
type CopyButtonProps = Readonly<{ value: string; label?: string }>;

export function CopyButton({ value, label = "コピー" }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);
  async function onCopy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);   // 1.2秒で元に戻す
    } catch { /* clipboard 不可環境では無反応（握りつぶし） */ }
  }
  return (
    <button type="button" className="btn btn-copy" onClick={() => void onCopy()}
            title={value} aria-label={`${label}: ${value}`}>
      {copied ? "✓ コピー済" : label}
    </button>
  );
}
```

仕様:
- `navigator.clipboard.writeText` を使用。成功で 1.2 秒だけ「✓ コピー済」へ変化し自動で戻る。
- クリップボード不可（古いブラウザ／非セキュアコンテキスト）では例外を握りつぶす（テスト画面のため致命的でない）。
- `title`／`aria-label` に実値を入れてホバー確認とアクセシビリティを確保。

設置箇所（2つ）:
1. `CreateMintPanel`: 生成 mint アドレスを `<code class="addr">{addr}</code>` ＋ `<CopyButton value={addr} />` で表示。
2. `AccountInfo`: ウォレットアドレス行を同様に `addr` ＋ `CopyButton` で表示。

アドレスは全長表示のまま（後続で貼り付けるため省略しない）。`shortenAddress` での短縮はしない。

## レイアウト整合（App.tsx）

`App.tsx` の `<main>` インライン style を `className="app"` に置換。`.app` で `text-align: left` を明示し `#root` の中央寄せに勝たせる。各 `<section>` 見出しに区切り線を入れドメイン境界を視認しやすくする。先頭の説明文は `color: var(--text)` のクラスに。`<h1>` は既存 `index.css` の大見出しスタイルがそのまま効くため変更不要。

## 検証（CLAUDE.md の正に従う）

- `cd app && pnpm build && pnpm test && pnpm lint` が通ること。生成コード（`app/src/generated`）は lint 対象外。
- localnet（surfpool）での目視確認:
  - 各パネルが新スタイル（枠・影・フォーカスリング・ボタン主従）で表示される。
  - mint 生成 → コピー → mintTo パネルへ貼り付けが動作する。
  - ウォレットアドレスのコピーが動作する。
  - ダークモードでも配色が破綻しない。
- IDL 変更なしのため `pnpm codegen` は不要。
