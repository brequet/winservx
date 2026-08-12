use crate::contract::events::{ServiceConfigChanged, ServiceStatusChanged, ServicesChanged};

/// Events produced by the liveness pipeline, routed to the frontend sink.
#[derive(Debug, Clone)]
pub enum LivenessEvent {
    Status(ServiceStatusChanged),
    Config(ServiceConfigChanged),
    Services(ServicesChanged),
}
