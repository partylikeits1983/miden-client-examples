# Miden Client Playground 

### Setup
1)  Install Miden Node:
```
cargo install miden-node --locked --features testing
```

2) Run the node:
```
miden-node make-genesis \
  --inputs-path  node/config/genesis.toml \
  --output-path node/storage/genesis.dat

cd node/storage
miden-node start \
--config node/config/miden-node.toml \
node
```

3) In new terminal window:
```
cargo run --release --bin mint_consume_example
```

### Reset Miden Node:
```
rm -rf *.sqlite3 
rm -rf node/storage/accounts
rm -rf node/storage/blocks
```








Dev Notes:

1) Create a new miden account
```
miden new-wallet
```

2) Create a new miden faucet
```
miden new-faucet -s public -t BTC -d 8 -m 21000000000000000
```

```
miden sync
```

3) Mint tokens from the faucet to your created account (replace with your wallet & faucet id)
```
miden mint --target 0x80ed046bc511a83c --asset 1000::0x2a021ef7df708360 --note-type private 
```

4) Run the script: 
```
cargo run --package client-test --bin send_p2id
```

5) Use the CLI to consume the P2ID

```
miden sync
miden consume-notes
```

Send note command
```
miden send --sender 0x80ed046bc511a83c --target 0x8d03743cf76f4a95 --asset 10::0x2a021ef7df708360 --note-type private
```