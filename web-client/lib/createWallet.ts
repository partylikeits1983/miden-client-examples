import { WebClient, AccountStorageMode } from '@demox-labs/miden-sdk';

export async function createWallet(): Promise<string> {
  const webClient = new WebClient();
  await webClient.create_client('http://localhost:57291');

  const wallet = await webClient.new_wallet(AccountStorageMode.private(), true);
 
  console.log("wallet id", wallet.id().to_string());
  return wallet.id().to_string();
}
