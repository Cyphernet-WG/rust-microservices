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

mod bus;
mod controller;

pub use bus::{BusConfig, BusId, ServiceAddress};
pub use controller::{Controller, EndpointList, Handler, PollItem};
use internet2::{presentation, transport};

/// Errors happening with RPC APIs
#[derive(Clone, Debug, Display, Error, From)]
#[display(doc_comments)]
pub enum Error<A: ServiceAddress> {
    /// unexpected server response
    UnexpectedServerResponse,

    /// message serialization or structure error. Details: {0}
    #[from(lightning_encoding::Error)]
    Presentation(presentation::Error),

    /// error sending message from {0} to {1}. Details: {2}
    Send(A, A, transport::Error),

    /// transport-level protocol error. Details: {0}
    #[from]
    Transport(transport::Error),

    /// provided service bus id {0} is unknown
    UnknownBusId(String),

    /// {0}
    ServiceError(String),
}

impl<A: ServiceAddress> From<zmq::Error> for Error<A> {
    fn from(err: zmq::Error) -> Self { Error::Transport(transport::Error::from(err)) }
}

impl<A: ServiceAddress> From<presentation::Error> for Error<A> {
    fn from(err: presentation::Error) -> Self {
        match err {
            presentation::Error::Transport(err) => err.into(),
            err => Error::Presentation(err),
        }
    }
}
