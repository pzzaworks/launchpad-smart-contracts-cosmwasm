use cosmwasm_schema::write_api;
use cw20_base::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }
}