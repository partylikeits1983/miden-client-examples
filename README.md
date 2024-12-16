# Miden Client Playground 

Prerequisites: Set up the miden node locally following the directions here: https://github.com/0xPolygonMiden/miden-node?tab=readme-ov-file#setup


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


#### Misc

Send note command
```
miden send --sender 0x80ed046bc511a83c --target 0x8d03743cf76f4a95 --asset 10::0x2a021ef7df708360 --note-type private
```