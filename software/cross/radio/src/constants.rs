//TODO Not much here - maybe move

//use static_assertions::{self, const_assert};

// URL where the list of stations are for rusty-radio
pub const STATIONS_URL: &str = "http://andrew-doble.hier-im-netz.de/ir/rr-stations.txt";

//pub const NUMBER_SOCKETS_STACK_RESOURCES: usize = 3;
// Need double the number of reseources (from the usual 3) as we are setting up two sockets:
//  - one for the audio streaming
//  - one to read the station list
pub const NUMBER_SOCKETS_STACK_RESOURCES: usize = 6;
//pub const NUMBER_SOCKETS_STACK_RESOURCES: usize = 9;
//pub const NUMBER_SOCKETS_STACK_RESOURCES: usize = 12;  // Used to work
//pub const NUMBER_SOCKETS_TCP_CLIENT_STATE: usize = 6;

// The number of sockets specified for StackResources needs to be the same or higher then the number of sockets specified
// in setting up the TcpClientState. Getting this wrong results in the program crashing - and took me a long time
// to figure out the cause.
// This is checked at compilation time by this macro.
// An alternative would be to use the same constant for setting up both StackResources and TcpClientState
//const_assert!(NUMBER_SOCKETS_STACK_RESOURCES >= NUMBER_SOCKETS_TCP_CLIENT_STATE);

// The address of the mcp23s17 device. This is hardwared on the front panel.
pub const MULTIPLEXER_DEVICE_ADDR: u8 = 0x00;
