use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum BroadcastEvent {
    EntityAnimation {
        exclude_uuid: Uuid,
        entity_id: i32,
        animation: u8,
    },
    EntityMetadata {
        exclude_uuid: Uuid,
        entity_id: i32,
        metadata_bytes: Vec<u8>,
    },
}
