use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    CreateDenom {
        subdenom: String,
    },
    ChangeAdmin {
        denom: String,
        new_admin_address: String,
    },
    MintTokens {
        denom: String,
        amount: Uint128,
        mint_to_address: String,
    },
    BurnTokens {
        denom: String,
        amount: Uint128,
        burn_from_address: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetDenomResponse)]
    GetDenom {
        creator_address: String,
        subdenom: String,
    },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetDenomResponse {
    pub denom: String,
}
