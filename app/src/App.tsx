import { ConnectButton } from "./components/ConnectButton";
import { AccountInfo } from "./components/AccountInfo";
import { AirdropButton } from "./components/AirdropButton";

function App() {
  return (
    <main style={{ maxWidth: 640, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <h1>urthr-net</h1>
      <section>
        <h2>ウォレット</h2>
        <ConnectButton />
        <AccountInfo />
      </section>
      <section>
        <h2>localnet</h2>
        <AirdropButton />
      </section>
      <section>
        <h2>プログラム</h2>
      </section>
    </main>
  );
}

export default App;
