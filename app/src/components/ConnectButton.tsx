import { useWalletConnection } from "@solana/react-hooks";
import { shortenAddress } from "../lib/format";

export function ConnectButton() {
  const { connectors, connect, disconnect, wallet, status } = useWalletConnection();
  const address = wallet?.account.address;

  if (status === "connected" && address) {
    return (
      <div>
        <span>接続中: {shortenAddress(address)}</span>
        <button onClick={() => disconnect()}>切断</button>
      </div>
    );
  }
  if (connectors.length === 0) {
    return <p>wallet-standard対応ウォレット（Phantom等）が見つかりません。拡張機能をインストールしてください。</p>;
  }
  return (
    <div>
      {connectors.map((connector) => (
        <button key={connector.id} onClick={() => connect(connector.id)}>
          {connector.name} で接続
        </button>
      ))}
    </div>
  );
}
