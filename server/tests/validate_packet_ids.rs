use serde_json::Value;

const PACKETS_JSON: &str = include_str!("../../docs/packets_26.1.2.json");

fn get_protocol_id(json: &Value, phase: &str, direction: &str, name: &str) -> i32 {
    let key = format!("minecraft:{name}");
    json[phase][direction][&key]["protocol_id"]
        .as_i64()
        .unwrap_or_else(|| panic!("Missing packet: {phase}/{direction}/{key}")) as i32
}

#[test]
fn validate_packet_ids_against_official_report() {
    let json: Value =
        serde_json::from_str(PACKETS_JSON).expect("Failed to parse packets_26.1.2.json");

    use rustmc_server::protocol::packet_ids::*;

    // Handshake serverbound
    assert_eq!(
        handshake::serverbound::HANDSHAKE,
        get_protocol_id(&json, "handshake", "serverbound", "intention"),
        "handshake/serverbound/intention mismatch"
    );

    // Status clientbound
    assert_eq!(
        status::clientbound::STATUS_RESPONSE,
        get_protocol_id(&json, "status", "clientbound", "status_response"),
        "status/clientbound/status_response mismatch"
    );
    assert_eq!(
        status::clientbound::PONG_RESPONSE,
        get_protocol_id(&json, "status", "clientbound", "pong_response"),
        "status/clientbound/pong_response mismatch"
    );

    // Status serverbound
    assert_eq!(
        status::serverbound::STATUS_REQUEST,
        get_protocol_id(&json, "status", "serverbound", "status_request"),
        "status/serverbound/status_request mismatch"
    );
    assert_eq!(
        status::serverbound::PING_REQUEST,
        get_protocol_id(&json, "status", "serverbound", "ping_request"),
        "status/serverbound/ping_request mismatch"
    );

    // Login clientbound
    assert_eq!(
        login::clientbound::LOGIN_SUCCESS,
        get_protocol_id(&json, "login", "clientbound", "login_finished"),
        "login/clientbound/login_finished mismatch"
    );
    assert_eq!(
        login::clientbound::SET_COMPRESSION,
        get_protocol_id(&json, "login", "clientbound", "login_compression"),
        "login/clientbound/login_compression mismatch"
    );
    assert_eq!(
        login::clientbound::COOKIE_REQUEST,
        get_protocol_id(&json, "login", "clientbound", "cookie_request"),
        "login/clientbound/cookie_request mismatch"
    );

    // Login serverbound
    assert_eq!(
        login::serverbound::LOGIN_START,
        get_protocol_id(&json, "login", "serverbound", "hello"),
        "login/serverbound/hello mismatch"
    );
    assert_eq!(
        login::serverbound::LOGIN_ACKNOWLEDGED,
        get_protocol_id(&json, "login", "serverbound", "login_acknowledged"),
        "login/serverbound/login_acknowledged mismatch"
    );
    assert_eq!(
        login::serverbound::COOKIE_RESPONSE,
        get_protocol_id(&json, "login", "serverbound", "cookie_response"),
        "login/serverbound/cookie_response mismatch"
    );

    // Configuration clientbound
    assert_eq!(
        configuration::clientbound::COOKIE_REQUEST,
        get_protocol_id(&json, "configuration", "clientbound", "cookie_request"),
        "configuration/clientbound/cookie_request mismatch"
    );
    assert_eq!(
        configuration::clientbound::FINISH_CONFIGURATION,
        get_protocol_id(
            &json,
            "configuration",
            "clientbound",
            "finish_configuration"
        ),
        "configuration/clientbound/finish_configuration mismatch"
    );
    assert_eq!(
        configuration::clientbound::REGISTRY_DATA,
        get_protocol_id(&json, "configuration", "clientbound", "registry_data"),
        "configuration/clientbound/registry_data mismatch"
    );
    assert_eq!(
        configuration::clientbound::STORE_COOKIE,
        get_protocol_id(&json, "configuration", "clientbound", "store_cookie"),
        "configuration/clientbound/store_cookie mismatch"
    );
    assert_eq!(
        configuration::clientbound::UPDATE_TAGS,
        get_protocol_id(&json, "configuration", "clientbound", "update_tags"),
        "configuration/clientbound/update_tags mismatch"
    );
    assert_eq!(
        configuration::clientbound::KNOWN_PACKS,
        get_protocol_id(&json, "configuration", "clientbound", "select_known_packs"),
        "configuration/clientbound/select_known_packs mismatch"
    );

    // Configuration serverbound
    assert_eq!(
        configuration::serverbound::COOKIE_RESPONSE,
        get_protocol_id(&json, "configuration", "serverbound", "cookie_response"),
        "configuration/serverbound/cookie_response mismatch"
    );
    assert_eq!(
        configuration::serverbound::ACKNOWLEDGE_FINISH,
        get_protocol_id(
            &json,
            "configuration",
            "serverbound",
            "finish_configuration"
        ),
        "configuration/serverbound/finish_configuration mismatch"
    );
    assert_eq!(
        configuration::serverbound::KNOWN_PACKS,
        get_protocol_id(&json, "configuration", "serverbound", "select_known_packs"),
        "configuration/serverbound/select_known_packs mismatch"
    );

    // Play clientbound
    assert_eq!(
        play::clientbound::CHUNK_BATCH_FINISHED,
        get_protocol_id(&json, "play", "clientbound", "chunk_batch_finished"),
        "play/clientbound/chunk_batch_finished mismatch"
    );
    assert_eq!(
        play::clientbound::CHUNK_BATCH_START,
        get_protocol_id(&json, "play", "clientbound", "chunk_batch_start"),
        "play/clientbound/chunk_batch_start mismatch"
    );
    assert_eq!(
        play::clientbound::COOKIE_REQUEST,
        get_protocol_id(&json, "play", "clientbound", "cookie_request"),
        "play/clientbound/cookie_request mismatch"
    );
    assert_eq!(
        play::clientbound::UNLOAD_CHUNK,
        get_protocol_id(&json, "play", "clientbound", "forget_level_chunk"),
        "play/clientbound/forget_level_chunk mismatch"
    );
    assert_eq!(
        play::clientbound::GAME_EVENT,
        get_protocol_id(&json, "play", "clientbound", "game_event"),
        "play/clientbound/game_event mismatch"
    );
    assert_eq!(
        play::clientbound::KEEP_ALIVE,
        get_protocol_id(&json, "play", "clientbound", "keep_alive"),
        "play/clientbound/keep_alive mismatch"
    );
    assert_eq!(
        play::clientbound::LEVEL_CHUNK_WITH_LIGHT,
        get_protocol_id(&json, "play", "clientbound", "level_chunk_with_light"),
        "play/clientbound/level_chunk_with_light mismatch"
    );
    assert_eq!(
        play::clientbound::LOGIN_PLAY,
        get_protocol_id(&json, "play", "clientbound", "login"),
        "play/clientbound/login mismatch"
    );
    assert_eq!(
        play::clientbound::PLAYER_INFO_UPDATE,
        get_protocol_id(&json, "play", "clientbound", "player_info_update"),
        "play/clientbound/player_info_update mismatch"
    );
    assert_eq!(
        play::clientbound::SYNCHRONIZE_PLAYER_POSITION,
        get_protocol_id(&json, "play", "clientbound", "player_position"),
        "play/clientbound/player_position mismatch"
    );
    assert_eq!(
        play::clientbound::SET_CENTER_CHUNK,
        get_protocol_id(&json, "play", "clientbound", "set_chunk_cache_center"),
        "play/clientbound/set_chunk_cache_center mismatch"
    );
    assert_eq!(
        play::clientbound::STORE_COOKIE,
        get_protocol_id(&json, "play", "clientbound", "store_cookie"),
        "play/clientbound/store_cookie mismatch"
    );
    assert_eq!(
        play::clientbound::SYSTEM_CHAT_MESSAGE,
        get_protocol_id(&json, "play", "clientbound", "system_chat"),
        "play/clientbound/system_chat mismatch"
    );
    assert_eq!(
        play::clientbound::TRANSFER,
        get_protocol_id(&json, "play", "clientbound", "transfer"),
        "play/clientbound/transfer mismatch"
    );

    // Play serverbound
    assert_eq!(
        play::serverbound::CONFIRM_TELEPORTATION,
        get_protocol_id(&json, "play", "serverbound", "accept_teleportation"),
        "play/serverbound/accept_teleportation mismatch"
    );
    assert_eq!(
        play::serverbound::CHAT_COMMAND,
        get_protocol_id(&json, "play", "serverbound", "chat_command"),
        "play/serverbound/chat_command mismatch"
    );
    assert_eq!(
        play::serverbound::CHAT_MESSAGE,
        get_protocol_id(&json, "play", "serverbound", "chat"),
        "play/serverbound/chat mismatch"
    );
    assert_eq!(
        play::serverbound::CHUNK_BATCH_RECEIVED,
        get_protocol_id(&json, "play", "serverbound", "chunk_batch_received"),
        "play/serverbound/chunk_batch_received mismatch"
    );
    assert_eq!(
        play::serverbound::CLIENT_TICK_END,
        get_protocol_id(&json, "play", "serverbound", "client_tick_end"),
        "play/serverbound/client_tick_end mismatch"
    );
    assert_eq!(
        play::serverbound::COOKIE_RESPONSE,
        get_protocol_id(&json, "play", "serverbound", "cookie_response"),
        "play/serverbound/cookie_response mismatch"
    );
    assert_eq!(
        play::serverbound::KEEP_ALIVE,
        get_protocol_id(&json, "play", "serverbound", "keep_alive"),
        "play/serverbound/keep_alive mismatch"
    );
    assert_eq!(
        play::serverbound::SET_PLAYER_POSITION,
        get_protocol_id(&json, "play", "serverbound", "move_player_pos"),
        "play/serverbound/move_player_pos mismatch"
    );
    assert_eq!(
        play::serverbound::SET_PLAYER_POSITION_AND_ROTATION,
        get_protocol_id(&json, "play", "serverbound", "move_player_pos_rot"),
        "play/serverbound/move_player_pos_rot mismatch"
    );
    assert_eq!(
        play::serverbound::SET_PLAYER_ROTATION,
        get_protocol_id(&json, "play", "serverbound", "move_player_rot"),
        "play/serverbound/move_player_rot mismatch"
    );
    assert_eq!(
        play::serverbound::SET_PLAYER_STATUS_ONLY,
        get_protocol_id(&json, "play", "serverbound", "move_player_status_only"),
        "play/serverbound/move_player_status_only mismatch"
    );
    assert_eq!(
        play::serverbound::PLAYER_COMMAND,
        get_protocol_id(&json, "play", "serverbound", "player_command"),
        "play/serverbound/player_command mismatch"
    );
    assert_eq!(
        play::serverbound::PLAYER_LOADED,
        get_protocol_id(&json, "play", "serverbound", "player_loaded"),
        "play/serverbound/player_loaded mismatch"
    );
    assert_eq!(
        play::serverbound::SET_CARRIED_ITEM,
        get_protocol_id(&json, "play", "serverbound", "set_carried_item"),
        "play/serverbound/set_carried_item mismatch"
    );
    assert_eq!(
        play::serverbound::SWING,
        get_protocol_id(&json, "play", "serverbound", "swing"),
        "play/serverbound/swing mismatch"
    );
}
