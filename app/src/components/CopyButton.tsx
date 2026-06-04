import { useState } from "react";

export type CopyButtonProps = Readonly<{
  /** クリップボードへ書き込む実値（アドレス等）。 */
  value: string;
  /** ボタン表示ラベル。既定は「コピー」。 */
  label?: string;
}>;

/**
 * 値をクリップボードへコピーする小さなボタン。成功すると 1.2 秒だけ
 * 「✓ コピー済」に変化し自動で戻る。クリップボード不可（古いブラウザや
 * 非セキュアコンテキスト）では例外を握りつぶす（テスト画面のため致命的でない）。
 */
export function CopyButton({ value, label = "コピー" }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      // clipboard 不可環境では無反応
    }
  }

  return (
    <button
      type="button"
      className="btn btn-copy"
      onClick={() => void onCopy()}
      title={value}
      aria-label={`${label}: ${value}`}
    >
      {copied ? "✓ コピー済" : label}
    </button>
  );
}
