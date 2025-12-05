use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub enum InterNodeMessage {
    ServiceCheckMutation { check_id: Uuid },
    ShuttingDown { process_id: Uuid },
}
