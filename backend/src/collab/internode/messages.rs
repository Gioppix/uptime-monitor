use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum InterNodeMessage {
    ServiceCheckMutation { check_id: Uuid },
}
