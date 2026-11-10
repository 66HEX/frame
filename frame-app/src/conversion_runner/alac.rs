//! Request-local ALAC verification and private output ownership.

mod export;
mod output;
mod prepare;
mod process;
mod scans;
mod staging;

pub(super) use export::run;
