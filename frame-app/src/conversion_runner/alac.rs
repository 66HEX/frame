//! Request-local ALAC verification and private output ownership.

mod export;
mod output;
mod prepare;
mod process;
mod scans;
pub(super) mod staging;

pub(super) use export::run;
