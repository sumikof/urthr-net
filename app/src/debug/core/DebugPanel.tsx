import type { ReactNode } from "react";
import { useInstructionRunner, type BuildInstruction } from "./useInstructionRunner";

export type DebugPanelProps = Readonly<{
  /** Legend shown on the fieldset. */
  title: string;
  /** The arg/account input controls for this instruction. */
  children: ReactNode;
  /**
   * Builds the instruction to run. Receives a wallet-derived `TransactionSigner`
   * to embed into the instruction's signer account(s). Resolve PDAs/args here.
   */
  build: BuildInstruction;
  /** Disable simulate/send (e.g. while inputs are invalid). */
  disabled?: boolean;
}>;

const box: React.CSSProperties = {
  border: "1px solid #ccc",
  borderRadius: 6,
  padding: "0.75rem 1rem",
  margin: "0.75rem 0",
};

/**
 * Reusable simulate-before-send card. Renders the supplied inputs, a "シミュレート"
 * button, and an "承認して送信" button that is only enabled after a successful
 * simulation. Shows the simulation summary, the signature, or any error.
 */
export function DebugPanel({ title, children, build, disabled }: DebugPanelProps) {
  const runner = useInstructionRunner();
  const { phase, summary, signature, error } = runner;

  return (
    <fieldset style={box}>
      <legend style={{ fontWeight: 600 }}>{title}</legend>

      {children}

      <div style={{ marginTop: "0.5rem", display: "flex", gap: 8 }}>
        <button
          type="button"
          onClick={() => void runner.simulate(build)}
          disabled={disabled || runner.isSimulating}
        >
          {runner.isSimulating ? "シミュレート中…" : "シミュレート"}
        </button>
        <button
          type="button"
          onClick={() => void runner.send()}
          disabled={disabled || !runner.canSend || runner.isSending}
        >
          {runner.isSending ? "送信中…" : "承認して送信"}
        </button>
      </div>

      {summary && (
        <div style={{ marginTop: "0.5rem", fontSize: "0.9em" }}>
          <div style={{ color: summary.err ? "crimson" : "green" }}>
            {summary.err ? "シミュレーション: 失敗" : "シミュレーション: 成功"}
            {summary.unitsConsumed != null && (
              <span style={{ marginLeft: 8, color: "#444" }}>
                CU: {summary.unitsConsumed.toString()}
              </span>
            )}
          </div>
          {summary.logs && summary.logs.length > 0 && (
            <details style={{ marginTop: 4 }}>
              <summary>ログ ({summary.logs.length})</summary>
              <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0", fontSize: "0.8em" }}>
                {summary.logs.join("\n")}
              </pre>
            </details>
          )}
        </div>
      )}

      {signature && phase === "sent" && (
        <p style={{ color: "green", marginTop: "0.5rem", wordBreak: "break-all" }}>
          署名: {signature}
        </p>
      )}

      {error && phase === "error" && (
        <p style={{ color: "crimson", marginTop: "0.5rem", wordBreak: "break-all" }}>
          エラー: {error}
        </p>
      )}
    </fieldset>
  );
}
