use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use osmo_reflect::msg::{
    ChainResponse, ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg, RawResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    // messages
    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    // The possible return types for QueryMsg cases
    export_schema(&schema_for!(OwnerResponse), &out_dir);
    export_schema(&schema_for!(ChainResponse), &out_dir);
    export_schema(&schema_for!(RawResponse), &out_dir);
}
