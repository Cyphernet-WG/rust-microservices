// LNP/BP Core Library implementing LNPBP specifications & standards
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

//! LNP/BP library for building robust and scalable APIs, servers, nodes,
//! clinets and cli utilities with LNP protocol, ZMQ and multi-threading

#![feature(never_type)]
#![recursion_limit = "256"]
// Coding conventions
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    dead_code,
    missing_docs
)]

extern crate amplify;
#[macro_use]
extern crate amplify_derive;
#[macro_use]
extern crate lnpbp_derive;

// Support for node & node clients development (include API helpers)
#[cfg(feature = "async")]
#[macro_use]
extern crate async_trait;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde_with;
#[cfg(feature = "serde")]
extern crate serde_crate as serde;

#[cfg(feature = "clap")]
#[macro_use]
extern crate clap;

pub mod error;
#[cfg(feature = "cli")]
pub mod format;
#[cfg(feature = "node")]
pub mod node;
#[cfg(any(feature = "client", feature = "node"))]
pub mod rpc;
#[cfg(feature = "shell")]
pub mod shell;
