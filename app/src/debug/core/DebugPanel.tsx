import { useEffect, type ReactNode } from "react";
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
  /**
   * When this value changes (e.g. any input field is edited), drop any stale
   * simulation so the user must re-simulate before sending. Pass a JSON.stringify
   * of all input state values. Prevents sending a tx built from old inputs.
   */
  resetKey?: unknown;
}>;


/**
 * Reusable simulate-before-send card. Renders the supplied inputs, a "シミュレート"
 * button, and an "承認して送信" button that is only enabled after a successful
 * simulation. Shows the simulation summary, the signature, or any error.
 */
export function DebugPanel({ title, children, build, disabled, resetKey }: DebugPanelProps) {
  const runner = useInstructionRunner();
  const { phase, summary, signature, error } = runner;

  // When the panel's inputs change (resetKey), drop any stale simulation so the
  // user must re-simulate before sending — prevents sending a tx built from old inputs.
  useEffect(() => {
    runner.reset();
    // Only react to input changes; runner.reset identity is stable enough and
    // including it would reset on unrelated rerenders.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  return (
    <fieldset className="panel">
      <legend>{title}</legend>

      {children}

      <div className="panel-actions">
        <button
          type="button"
          className="btn"
          onClick={() => void runner.simulate(build)}
          disabled={disabled || runner.isSimulating}
        >
          {runner.isSimulating ? "シミュレート中…" : "シミュレート"}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void runner.send()}
          disabled={disabled || !runner.canSend || runner.isSending}
        >
          {runner.isSending ? "送信中…" : "承認して送信"}
        </button>
      </div>

      {summary && (
        <div className="result">
          <div className={summary.err ? "result-err" : "result-ok"}>
            {summary.err ? "シミュレーション: 失敗" : "シミュレーション: 成功"}
            {summary.unitsConsumed != null && (
              <span style={{ marginLeft: 8, color: "var(--text)" }}>
                CU: {summary.unitsConsumed.toString()}
              </span>
            )}
          </div>
          {summary.logs && summary.logs.length > 0 && (
            <details style={{ marginTop: 4 }}>
              <summary>ログ ({summary.logs.length})</summary>
              <pre>{summary.logs.join("\n")}</pre>
            </details>
          )}
        </div>
      )}

      {signature && phase === "sent" && (
        <p className="result result-ok" style={{ wordBreak: "break-all" }}>
          署名: {signature}
        </p>
      )}

      {error && phase === "error" && (
        <p
          className="result result-err"
          style={{ wordBreak: "break-all", whiteSpace: "pre-wrap" }}
        >
          エラー: {error}
        </p>
      )}

      {(phase === "sent" || phase === "error") && (
        <div style={{ marginTop: "0.5rem" }}>
          <button type="button" className="btn" onClick={() => runner.reset()}>
            リセット
          </button>
        </div>
      )}
    </fieldset>
  );
}
