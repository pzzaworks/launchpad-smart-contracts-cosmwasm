#[cfg(test)]
mod test {
    use crate::contract::*;
    use cosmwasm_std::{Uint128, MessageInfo, Env, Response};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw20::{Cw20Coin, TokenInfoResponse};
    use cw20_base::contract::query_token_info;
    use cw20_base::msg::InstantiateMsg;

    #[test]
    fn instantiate_test() {
        let mut deps = mock_dependencies();
        let env : Env            = mock_env();
        let info: MessageInfo    = mock_info(&"sender", &[]);
        let msg : InstantiateMsg = InstantiateMsg {
            name             : "Test Token".to_string(),
            symbol           : "TEST".to_string(),
            decimals         : 6,
            initial_balances : vec![
                Cw20Coin {
                    address  : String::from("sender"),
                    amount   : Uint128::new(1928334),
                }
            ],
            mint             : None,
            marketing        : None,
        };
        let res: Response = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Test Token".to_string(),
                symbol: "TEST".to_string(),
                decimals: 6,
                total_supply: Uint128::new(1928334),
            }
        );
    }
}
