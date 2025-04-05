use static_assertions::{self, const_assert};

pub const NUMBER_SOCKETS_STACK_RESOURCES: usize = 3;
pub const NUMBER_SOCKETS_TCP_CLIENT_STATE: usize = 3;

// The number of sockets specified for StackResources needs to be the same or higher then the number of sockets specified
// in setting up the TcpClientState. Getting this wrong results in the program crashing - and took me a long time
// to figure out the cause.
// This is checked at compilation time by this macro.
// An alternative would be to use the same constant for setting up both StackResources and TcpClientState
const_assert!(NUMBER_SOCKETS_STACK_RESOURCES >= NUMBER_SOCKETS_TCP_CLIENT_STATE);
