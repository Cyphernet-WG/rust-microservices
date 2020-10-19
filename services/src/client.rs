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

use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::transport::zmqsocket::{ApiType, SocketLocator};
use lnpbp::lnp::{
    transport, CreateUnmarshaller, NoEncryption, Session, Unmarshall,
    Unmarshaller,
};

use crate::rpc;

pub struct RpcClient<Endpoints, Api>
where
    Api: rpc::Api,
    Endpoints: rpc::EndpointTypes,
{
    sessions: HashMap<
        Endpoints,
        Session<NoEncryption, transport::zmqsocket::Connection>,
    >,
    unmarshaller: Unmarshaller<Api::Reply>,
}

impl<Endpoints, Api> RpcClient<Endpoints, Api>
where
    Api: rpc::Api,
    Endpoints: rpc::EndpointTypes,
{
    pub fn init(
        endpoints: HashMap<Endpoints, SocketLocator>,
        context: &zmq::Context,
    ) -> Result<Self, transport::Error> {
        let mut sessions: HashMap<Endpoints, Session<_, _>> = none!();
        for (service, endpoint) in endpoints {
            sessions.insert(
                service,
                Session::new_zmq_unencrypted(
                    ApiType::Client,
                    &context,
                    endpoint,
                    None,
                )?,
            );
        }
        let unmarshaller = Api::Reply::create_unmarshaller();
        Ok(Self {
            sessions,
            unmarshaller,
        })
    }

    pub fn request(
        &mut self,
        endpoint: Endpoints,
        request: Api::Request,
    ) -> Result<Api::Reply, rpc::Error> {
        let data = request.encode()?;
        let connection = self
            .sessions
            .get_mut(&endpoint)
            .ok_or(rpc::Error::UnknownEndpoint(endpoint.to_string()))?;
        connection.send_raw_message(data)?;
        let raw = connection.recv_raw_message()?;
        let reply = self.unmarshaller.unmarshall(&raw)?;
        Ok((&*reply).clone())
    }
}
