import { useState } from "react";
import { createWallet } from "../lib/createWallet";
import { setupFaucet } from "../lib/createFaucet";

function App() {
  const [walletId, setWalletId] = useState<string | null>(null);
  const [faucetId, setFaucetId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleCreateWallet = async () => {
    try {
      const id = await createWallet();
      setWalletId(id);
      console.log("Wallet ID:", id);
    } catch (err: any) {
      console.error(err);
      setError(err.message || "Error creating wallet");
    }
  };

  const handleSetupFaucet = async () => {
    try {
      const id = await setupFaucet();
      setFaucetId(id);
      console.log("Faucet ID:", id);
    } catch (err: any) {
      console.error(err);
      setError(err.message || "Error setting up faucet");
    }
  };

  return (
    <div style={{ padding: "20px" }}>
      <h1>Miden SDK Demo</h1>
      <div style={{ display: "flex", flexDirection: "column", gap: "15px", maxWidth: "300px" }}>
        {/* Wallet Creation */}
        <button onClick={handleCreateWallet}>Create Wallet</button>
        {walletId && <p>Wallet created with ID: {walletId}</p>}

        {/* Faucet Setup */}
        <button onClick={handleSetupFaucet}>Setup Faucet</button>
        {faucetId && <p>Faucet created with ID: {faucetId}</p>}
      </div>

      {/* Error Display */}
      {error && <p style={{ color: "red", marginTop: "20px" }}>{error}</p>}
    </div>
  );
}

export default App;
