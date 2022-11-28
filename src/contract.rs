use apollo_utils::assets::assert_only_native_coins;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_asset::{Asset, AssetList};
use cw_dex::osmosis::OsmosisPool;
use cw_dex::traits::Pool;

use crate::error::ContractError;
use crate::msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:osmosis-liquidity-helper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BalancingProvideLiquidity {
            assets,
            min_out,
            pool,
            recipient,
        } => {
            let assets = assets.check(deps.api)?;
            let pool: OsmosisPool = from_binary(&pool)?;
            execute_balancing_provide_liquidity(deps, env, info, assets, min_out, pool, recipient)
        }
        ExecuteMsg::Callback(msg) => {
            // Only contract can call callbacks
            if info.sender != env.contract.address {
                return Err(ContractError::Unauthorized {});
            }

            match msg {
                CallbackMsg::SingleSidedJoin {
                    asset,
                    pool,
                    recipient,
                } => execute_callback_single_sided_join(deps, env, info, asset, pool, recipient),
                CallbackMsg::ReturnLpTokens {
                    pool,
                    balance_before,
                    recipient,
                } => execute_callback_return_lp_tokens(
                    deps,
                    env,
                    info,
                    pool,
                    balance_before,
                    recipient,
                ),
            }
        }
    }
}

pub fn execute_balancing_provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut assets: AssetList,
    min_out: Uint128,
    pool: OsmosisPool,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    // Assert that only native coins are sent
    assert_only_native_coins(&assets)?;

    // Unwrap recipient or use caller's address
    let recipient = recipient.map_or(Ok(info.sender), |x| deps.api.addr_validate(&x))?;

    if assets.len() == 1 {
        let lp_token_balance = pool
            .lp_token()
            .query_balance(&deps.querier, env.contract.address.to_string())?;

        // Provide single sided
        let provide_res = pool.provide_liquidity(deps.as_ref(), &env, assets.clone(), min_out)?;

        // Callback to return LP tokens
        let callback_msg = CallbackMsg::ReturnLpTokens {
            pool,
            balance_before: lp_token_balance,
            recipient,
        }
        .into_cosmos_msg(&env)?;

        let event =
            Event::new("apollo/osmosis-liquidity-helper/execute_balancing_provide_liquidity")
                .add_attribute("action", "single_sided_provide_liquidity")
                .add_attribute("assets", assets.to_string());

        Ok(provide_res.add_message(callback_msg).add_event(event))
    } else {
        // Provide as much as possible double sided, and then issue callbacks to
        // provide the remainder single sided
        let (lp_tokens_received, tokens_used) =
            pool.simulate_noswap_join(&deps.querier, &assets)?;

        // Get response with msg to provide double sided
        let mut response =
            pool.provide_liquidity(deps.as_ref(), &env, assets.clone(), lp_tokens_received)?;

        // Deduct tokens used to get remaining tokens
        assets.deduct_many(&tokens_used)?;

        // For each of the remaining tokens, issue a callback to provide
        // liquidity single sided
        for asset in assets.into_iter() {
            if asset.amount > Uint128::zero() {
                let msg = CallbackMsg::SingleSidedJoin {
                    asset: asset.clone(),
                    pool,
                    recipient: recipient.clone(),
                }
                .into_cosmos_msg(&env)?;
                response = response.add_message(msg);
            }
        }

        let event =
            Event::new("apollo/osmosis-liquidity-helper/execute_balancing_provide_liquidity")
                .add_attribute("action", "double_sided_provide_liquidity")
                .add_attribute("assets", assets.to_string());

        Ok(response.add_event(event))
    }
}

/// CallbackMsg handler to provide liquidity with the given assets. This needs
/// to be a callback, rather than doing in the first ExecuteMsg, because
/// pool.provide_liquidity does a simulation with current reserves, and our
/// actions in the top level ExecuteMsg will alter the reserves, which means the
/// reserves would be wrong in the provide liquidity simulation.
pub fn execute_callback_single_sided_join(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset: Asset,
    pool: OsmosisPool,
    recipient: Addr,
) -> Result<Response, ContractError> {
    let assets = AssetList::from(vec![asset.clone()]);

    let lp_token_balance = pool
        .lp_token()
        .query_balance(&deps.querier, env.contract.address.to_string())?;

    let res = pool.provide_liquidity(deps.as_ref(), &env, assets, Uint128::one())?;

    let callback_msg = CallbackMsg::ReturnLpTokens {
        pool,
        balance_before: lp_token_balance,
        recipient,
    }
    .into_cosmos_msg(&env)?;

    let event = Event::new("apollo/osmosis-liquidity-helper/execute_callback_single_sided_join")
        .add_attribute("asset", asset.to_string());

    Ok(res.add_message(callback_msg).add_event(event))
}

pub fn execute_callback_return_lp_tokens(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    pool: OsmosisPool,
    balance_before: Uint128,
    recipient: Addr,
) -> Result<Response, ContractError> {
    let lp_token = pool.lp_token();
    let lp_token_balance = lp_token.query_balance(&deps.querier, env.contract.address)?;

    let return_amount = lp_token_balance.checked_sub(balance_before)?;
    let return_asset = Asset::new(lp_token, return_amount);
    let msg = return_asset.transfer_msg(&recipient)?;

    let event = Event::new("apollo/osmosis-liquidity-helper/execute_callback_return_lp_tokens")
        .add_attribute("return_asset", return_asset.to_string())
        .add_attribute("recipient", recipient);

    Ok(Response::new().add_message(msg).add_event(event))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!();
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {}
