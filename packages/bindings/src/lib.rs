mod msg;
mod query;

pub use msg::OsmosisMsg;
pub use query::{FullDenomResponse, OsmosisQuery};

// This is a signal, such that any contract that imports these helpers will only run on the
// osmosis blockchain
#[no_mangle]
extern "C" fn requires_osmosis() {}
