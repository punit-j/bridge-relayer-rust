# spectre-bridge-service

## installation
### requirements

### dependencies
* pkg-config
* make
* libssl-dev
* libclang-dev


## Proof
### Manual generation
Generation proof for the given `tx_hash`
```
call [eth_getTransactionReceipt](https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_gettransactionreceipt) for `blockHash` and `to`.
call [eth_getLogs](https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_getlogs)(address=to, blockHash=blockHash) with accquired from previous step `blockHash` and `to`

In the reposnse find transaction_hash=tx_hash and get `logIndex`.

Then call 
index.js eth-to-near-find-proof '{"logIndex": log_index, "transactionHash": tx_hash}' --eth-node-url rpc_url
(log_index should be in dec format)
```
