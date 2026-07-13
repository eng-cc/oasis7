use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::WorldError;
use oasis7_proto::distributed_net as proto_net;

pub use proto_net::{NetworkMessage, NetworkRequest, NetworkResponse, NetworkSubscription};

pub trait DistributedNetwork: proto_net::DistributedNetwork<WorldError> {}

impl<T> DistributedNetwork for T where T: proto_net::DistributedNetwork<WorldError> {}

#[derive(Clone, Default)]
pub struct InMemoryNetwork {
    inbox: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    published: Arc<Mutex<Vec<NetworkMessage>>>,
    handlers: Arc<Mutex<HashMap<String, Handler>>>,
}

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>;

impl InMemoryNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn published(&self) -> Vec<NetworkMessage> {
        self.published.lock().expect("lock published").clone()
    }
}

impl proto_net::DistributedNetwork<WorldError> for InMemoryNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        let message = NetworkMessage {
            topic: topic.to_string(),
            payload: payload.to_vec(),
        };
        {
            let mut published = self.published.lock().expect("lock published");
            published.push(message.clone());
        }
        proto_net::push_bounded_inbox_message(
            &self.inbox,
            topic,
            message.payload,
            proto_net::DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
        );
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        let mut inbox = self.inbox.lock().expect("lock inbox");
        inbox.entry(topic.to_string()).or_default();
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::clone(&self.inbox),
        ))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let handler = {
            let handlers = self.handlers.lock().expect("lock handlers");
            handlers.get(protocol).cloned()
        };
        let handler = handler.ok_or_else(|| WorldError::NetworkProtocolUnavailable {
            protocol: protocol.to_string(),
        })?;
        handler(payload)
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        let mut handlers = self.handlers.lock().expect("lock handlers");
        handlers.insert(protocol.to_string(), Arc::from(handler));
        Ok(())
    }

    fn register_context_handler_with_admission(
        &self,
        protocol: &str,
        admission: proto_net::NetworkAdmission<WorldError>,
        handler: proto_net::ContextNetworkHandler<WorldError>,
    ) -> Result<(), WorldError> {
        self.register_handler(
            protocol,
            Box::new(move |payload| {
                admission(payload)?;
                let context = proto_net::NetworkRequestContext::new(
                    std::time::Instant::now() + std::time::Duration::from_secs(30),
                    Arc::new(std::sync::atomic::AtomicBool::new(false)),
                );
                handler(&context, payload)
            }),
        )
    }
}
