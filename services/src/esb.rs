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

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::rpc_connection::Request;
use lnpbp::lnp::transport::zmqsocket;
use lnpbp::lnp::{
    presentation, session, transport, NoEncryption, Session, Unmarshall,
    Unmarshaller,
};

#[cfg(feature = "node")]
use crate::node::TryService;

/// Marker traits for service bus identifiers
pub trait BusId: Copy + Eq + Hash + Display {}

/// Marker traits for service bus identifiers
pub trait ServiceAddress:
    Copy + Eq + Hash + Debug + Display + AsRef<[u8]> + Into<Vec<u8>> + From<Vec<u8>>
{
}

/// Errors happening with RPC APIs
#[derive(Clone, Debug, Display, Error, From)]
#[display(doc_comments)]
pub enum Error {
    /// Unexpected server response
    UnexpectedServerResponse,

    /// Message serialization or structure error: {_0}
    Presentation(presentation::Error),

    /// Transport-level protocol error: {_0}
    #[from]
    Transport(transport::Error),

    /// The provided service bus id {_0} is unknown
    UnknownBusId(String),

    /// {_0}
    ServiceError(String),
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Self {
        Error::Transport(transport::Error::from(err))
    }
}

impl From<presentation::Error> for Error {
    fn from(err: presentation::Error) -> Self {
        match err {
            presentation::Error::Transport(err) => err.into(),
            err => Error::Presentation(err),
        }
    }
}

/// Trait for types handling specific set of ESB RPC API requests structured as
/// a single type implementing [`Request`].
pub trait Handler<B>
where
    Self: Sized,
    B: BusId,
    Error: From<Self::Error>,
{
    type Request: Request;
    type Address: ServiceAddress;
    type Error: std::error::Error;

    fn handle(
        &mut self,
        senders: &mut Senders<B>,
        bus_id: B,
        source: Self::Address,
        request: Self::Request,
    ) -> Result<(), Self::Error>;

    fn handle_err(&mut self, error: Error) -> Result<(), Error>;
}

pub struct Senders<B>
where
    B: BusId,
{
    pub(self) sessions:
        HashMap<B, session::Raw<NoEncryption, zmqsocket::Connection>>,
    pub(self) router: Vec<u8>,
}

impl<B> Senders<B>
where
    B: BusId,
{
    pub fn send_to<A, R>(
        &mut self,
        bus_id: B,
        dest: A,
        request: R,
    ) -> Result<(), Error>
    where
        A: ServiceAddress,
        R: Request,
    {
        trace!("Sending {} to {} via {}", request, dest, bus_id);
        let data = request.encode()?;
        let session = self
            .sessions
            .get_mut(&bus_id)
            .ok_or(Error::UnknownBusId(bus_id.to_string()))?;
        session.send_routed_message(
            self.router.as_ref(),
            dest.as_ref(),
            &data,
        )?;
        Ok(())
    }
}

pub struct Controller<B, R, H>
where
    R: Request,
    B: BusId,
    H: Handler<B, Request = R>,
    Error: From<H::Error>,
{
    identity: H::Address,
    senders: Senders<B>,
    unmarshaller: Unmarshaller<R>,
    handler: H,
}

impl<B, R, H> Controller<B, R, H>
where
    R: Request,
    B: BusId,
    H: Handler<B, Request = R>,
    Error: From<H::Error>,
{
    pub fn init(
        identity: H::Address,
        service_bus: HashMap<B, zmqsocket::Carrier>,
        router: H::Address,
        handler: H,
        api_type: zmqsocket::ApiType,
    ) -> Result<Self, transport::Error> {
        let mut sessions: HashMap<B, session::Raw<_, _>> = none!();
        for (service, carrier) in service_bus {
            let session = match carrier {
                zmqsocket::Carrier::Locator(locator) => {
                    debug!(
                        "Creating session for {} service located at {} with identity '{}'",
                        &service,
                        &locator,
                        &identity
                    );
                    let session = session::Raw::with_zmq_unencrypted(
                        api_type,
                        &locator,
                        None,
                        Some(identity.as_ref()),
                    )?;
                    session.as_socket().set_router_mandatory(true)?;
                    session
                }
                zmqsocket::Carrier::Socket(socket) => {
                    debug!("Creating session for {} service", &service);
                    session::Raw::from_zmq_socket_unencrypted(api_type, socket)
                }
            };
            sessions.insert(service, session);
        }
        let unmarshaller = R::create_unmarshaller();
        let senders = Senders {
            sessions,
            router: router.into(),
        };

        Ok(Self {
            identity,
            senders,
            unmarshaller,
            handler,
        })
    }

    pub fn send_to(
        &mut self,
        endpoint: B,
        dest: H::Address,
        request: R,
    ) -> Result<(), Error> {
        self.senders.send_to(endpoint, dest, request)
    }
}

#[cfg(feature = "node")]
impl<B, R, H> TryService for Controller<B, R, H>
where
    R: Request,
    B: BusId,
    H: Handler<B, Request = R>,
    Error: From<H::Error>,
{
    type ErrorType = Error;

    fn try_run_loop(mut self) -> Result<(), Self::ErrorType> {
        loop {
            match self.run() {
                Ok(_) => debug!("ESB request processing complete"),
                Err(err) => {
                    error!("ESB request processing error: {}", err);
                    self.handler.handle_err(err)?;
                }
            }
        }
    }
}

impl<B, R, H> Controller<B, R, H>
where
    R: Request,
    B: BusId,
    H: Handler<B, Request = R>,
    Error: From<H::Error>,
{
    fn run(&mut self) -> Result<(), Error> {
        let mut index = vec![];
        let mut items = self
            .senders
            .sessions
            .iter()
            .map(|(service, session)| {
                index.push(service);
                session.as_socket().as_poll_item(zmq::POLLIN | zmq::POLLERR)
            })
            .collect::<Vec<_>>();

        trace!("Awaiting for ESB request from {} services...", items.len());
        let _ = zmq::poll(&mut items, -1)?;

        let service_buses = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if item.get_revents().is_empty() {
                    None
                } else {
                    Some(*index[i])
                }
            })
            .collect::<Vec<_>>();
        trace!(
            "Received ESB request from {} services...",
            service_buses.len()
        );

        for bus_id in service_buses {
            let session = self
                .senders
                .sessions
                .get_mut(&bus_id)
                .expect("must exist, just indexed");

            let routed_frame = session.recv_routed_message()?;
            let request =
                (&*self.unmarshaller.unmarshall(&routed_frame.msg)?).clone();
            let source = H::Address::from(routed_frame.src);
            let dest = H::Address::from(routed_frame.dst);

            if dest == self.identity {
                // We are the destination
                debug!("ESB request from {}: {}", source, request);

                self.handler.handle(
                    &mut self.senders,
                    bus_id,
                    source,
                    request,
                )?;
            } else {
                // Need to route
                debug!("ESB request routed from {} to {}", source, dest);

                self.senders.send_to(bus_id, dest, request)?
            }
        }

        Ok(())
    }
}
