import { ConnectButton } from "./components/ConnectButton";
import { AccountInfo } from "./components/AccountInfo";
import { AirdropButton } from "./components/AirdropButton";
import { CreateMintPanel } from "./debug/localnet/CreateMintPanel";
import { MintToPanel } from "./debug/localnet/MintToPanel";
import { InitializeProtocolPanel } from "./debug/instructions/InitializeProtocolPanel";
import { RegisterPublisherPanel } from "./debug/instructions/RegisterPublisherPanel";
import { StakePanel } from "./debug/instructions/StakePanel";
import { UnstakePanel } from "./debug/instructions/UnstakePanel";
import { CreateCampaignPanel } from "./debug/instructions/CreateCampaignPanel";
import { FundCampaignPanel } from "./debug/instructions/FundCampaignPanel";
import { CloseCampaignPanel } from "./debug/instructions/CloseCampaignPanel";
import { SubmitClaimPanel } from "./debug/instructions/SubmitClaimPanel";
import { ChallengeClaimPanel } from "./debug/instructions/ChallengeClaimPanel";
import { ResolveClaimPanel } from "./debug/instructions/ResolveClaimPanel";
import { SettleClaimPanel } from "./debug/instructions/SettleClaimPanel";
import { ProtocolConfigInspector } from "./debug/accounts/ProtocolConfigInspector";
import { PublisherInspector } from "./debug/accounts/PublisherInspector";
import { CampaignInspector } from "./debug/accounts/CampaignInspector";
import { ClaimInspector } from "./debug/accounts/ClaimInspector";

function App() {
  return (
    <main style={{ maxWidth: 880, margin: "2rem auto", fontFamily: "sans-serif" }}>
      <h1>urthr-net — デバッグハーネス</h1>
      <p style={{ color: "#666" }}>
        localnet 用デバッグ画面。各命令をシミュレートしてから送信します。
      </p>
      <section>
        <h2>ウォレット</h2>
        <ConnectButton />
        <AccountInfo />
      </section>
      <section>
        <h2>localnet</h2>
        <AirdropButton />
        <CreateMintPanel />
        <MintToPanel />
      </section>
      <section>
        <h2>プロトコル設定</h2>
        <InitializeProtocolPanel />
        <ProtocolConfigInspector />
      </section>
      <section>
        <h2>パブリッシャー</h2>
        <RegisterPublisherPanel />
        <StakePanel />
        <UnstakePanel />
        <PublisherInspector />
      </section>
      <section>
        <h2>キャンペーン</h2>
        <CreateCampaignPanel />
        <FundCampaignPanel />
        <CloseCampaignPanel />
        <CampaignInspector />
      </section>
      <section>
        <h2>Claim ライフサイクル</h2>
        <SubmitClaimPanel />
        <ChallengeClaimPanel />
        <ResolveClaimPanel />
        <SettleClaimPanel />
        <ClaimInspector />
      </section>
    </main>
  );
}

export default App;
