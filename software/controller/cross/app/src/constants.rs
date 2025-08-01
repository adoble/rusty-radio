//TODO Not much here - maybe move

//use static_assertions::{self, const_assert};

pub const NUMBER_SOCKETS_STACK_RESOURCES: usize = 3;
//pub const NUMBER_SOCKETS_TCP_CLIENT_STATE: usize = 3;

// The number of sockets specified for StackResources needs to be the same or higher then the number of sockets specified
// in setting up the TcpClientState. Getting this wrong results in the program crashing - and took me a long time
// to figure out the cause.
// This is checked at compilation time by this macro.
// An alternative would be to use the same constant for setting up both StackResources and TcpClientState
//const_assert!(NUMBER_SOCKETS_STACK_RESOURCES >= NUMBER_SOCKETS_TCP_CLIENT_STATE);

// Constants around the creation and reading of stations.
pub const MAX_STATION_NAME_LEN: usize = 40;
pub const MAX_STATION_URL_LEN: usize = 256;
pub const NUMBER_PRESETS: usize = 4;

// The address of the mcp23s17 device. This is hardwared on the front panel.
pub const MULTIPLEXER_DEVICE_ADDR: u8 = 0x00;
