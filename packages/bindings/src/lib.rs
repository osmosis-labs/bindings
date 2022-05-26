mod msg;
mod query;
mod types;

pub use msg::OsmosisMsg;
pub use query::{
    FullDenomResponse, OsmosisQuery, PoolStateResponse, SpotPriceResponse, SwapResponse,
};
pub use types::{LockTokensResponse, Step, Swap, SwapAmount, SwapAmountWithLimit};

// This is a signal, such that any contract that imports these helpers will only run on the
// osmosis blockchain
#[no_mangle]
extern "C" fn requires_osmosis() {}
