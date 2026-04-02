/// This is an adapter for mcp23s17_async::Mcp23s17 which does not prvide a Send trait as required by the embassy tasks.
///
/// See [issue 1](https://github.com/Fundevoge/mcp23s17-async/issues/1) in the github repo for the mcp23s17-async crate.
///
/// The `Deref` and `DerefMut` implementations means hat it can be used a a drop in replacement for `Mcp23s17`.
use core::ops::{Deref, DerefMut};

use crate::MultiplexerDriverType;

pub struct SendableMultiplexerDriver(pub MultiplexerDriverType<'static>);

unsafe impl Send for SendableMultiplexerDriver {}

impl Deref for SendableMultiplexerDriver {
    type Target = MultiplexerDriverType<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SendableMultiplexerDriver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
