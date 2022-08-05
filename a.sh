#CHAIN ID
CHAIN_ID=osmo-test-4
#Signer
ACCOUNT=testnet8
#Delete account if already present
osmosisd keys delete $ACCOUNT --keyring-backend test
#Mnemonic
echo "bronze arctic poverty female latin monkey fork walnut tornado trophy dawn doll" | osmosisd keys add $ACCOUNT --keyring-backend test --recover
#Account Address
ACCOUNT_ADDRESS=$(osmosisd keys show -a $ACCOUNT --keyring-backend test)

#Account Address and Balance
echo Balance for $ACCOUNT_ADDRESS
osmosisd query bank balances $ACCOUNT_ADDRESS

#Contract Name
CONTRACT=ION_DAO
#Submit proposal and store wasm binary including a deposit amount
osmosisd tx gov submit-proposal wasm-store tokenfactory.wasm --title "Add ion_stake" \
  --description "Let's upload this contract" --run-as $ACCOUNT_ADDRESS \
  --from $ACCOUNT --keyring-backend test --chain-id $CHAIN_ID -y -b block \
  --gas 9000000 --gas-prices 0.025uosmo --deposit 500000000uosmo 

#🛑 STOP HERE 🛑, Modify value below. 
# Proposal ID from previous step
PROPOSAL=56886
