use uuid::Uuid;

pub const BROADCAST_RENDER_DISTANCE: i32 = 8;

pub fn is_within_render_distance(
    source_chunk_x: i32,
    source_chunk_z: i32,
    receiver_chunk_x: i32,
    receiver_chunk_z: i32,
) -> bool {
    let dx = (source_chunk_x - receiver_chunk_x).abs();
    let dz = (source_chunk_z - receiver_chunk_z).abs();
    dx <= BROADCAST_RENDER_DISTANCE && dz <= BROADCAST_RENDER_DISTANCE
}

#[derive(Debug, Clone)]
pub enum BroadcastEvent {
    EntityAnimation {
        exclude_uuid: Uuid,
        entity_id: i32,
        animation: u8,
        source_chunk_x: i32,
        source_chunk_z: i32,
    },
    EntityMetadata {
        exclude_uuid: Uuid,
        entity_id: i32,
        metadata_bytes: Vec<u8>,
        source_chunk_x: i32,
        source_chunk_z: i32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_within_render_distance_at_origin() {
        assert!(is_within_render_distance(0, 0, 0, 0));
    }

    #[test]
    fn test_within_render_distance_at_boundary() {
        assert!(is_within_render_distance(0, 0, 8, 8));
        assert!(is_within_render_distance(0, 0, -8, -8));
        assert!(is_within_render_distance(5, 5, 13, 13));
    }

    #[test]
    fn test_outside_render_distance() {
        assert!(!is_within_render_distance(0, 0, 9, 0));
        assert!(!is_within_render_distance(0, 0, 0, 9));
        assert!(!is_within_render_distance(0, 0, 9, 9));
    }

    #[test]
    fn test_render_distance_matches_view_distance() {
        assert_eq!(BROADCAST_RENDER_DISTANCE, 8);
    }
}
