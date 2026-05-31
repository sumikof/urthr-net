import { fetchMaybeProtocolConfig } from "../../generated";
import { configPda } from "../core/pdas";
import { AccountInspector } from "./AccountInspector";

export function ProtocolConfigInspector() {
  return (
    <AccountInspector
      title="ProtocolConfig"
      defaultAddress={() => configPda()}
      fetchMaybe={fetchMaybeProtocolConfig}
    />
  );
}
