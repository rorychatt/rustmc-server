use uuid::Uuid;

pub fn is_within_render_distance(
    source_chunk_x: i32,
    source_chunk_z: i32,
    receiver_chunk_x: i32,
    receiver_chunk_z: i32,
    view_distance: i32,
) -> bool {
    let dx = (source_chunk_x - receiver_chunk_x).abs();
    let dz = (source_chunk_z - receiver_chunk_z).abs();
    dx <= view_distance && dz <= view_distance
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
        assert!(is_within_render_distance(0, 0, 0, 0, 8));
    }

    #[test]
    fn test_within_render_distance_at_boundary() {
        assert!(is_within_render_distance(0, 0, 8, 8, 8));
        assert!(is_within_render_distance(0, 0, -8, -8, 8));
        assert!(is_within_render_distance(5, 5, 13, 13, 8));
    }

    #[test]
    fn test_outside_render_distance() {
        assert!(!is_within_render_distance(0, 0, 9, 0, 8));
        assert!(!is_within_render_distance(0, 0, 0, 9, 8));
        assert!(!is_within_render_distance(0, 0, 9, 9, 8));
    }

    #[test]
    fn test_custom_view_distance() {
        // View distance 4: boundary at 4 chunks
        assert!(is_within_render_distance(0, 0, 4, 4, 4));
        assert!(!is_within_render_distance(0, 0, 5, 0, 4));

        // View distance 16: boundary at 16 chunks
        assert!(is_within_render_distance(0, 0, 16, 16, 16));
        assert!(!is_within_render_distance(0, 0, 17, 0, 16));
    }
}
