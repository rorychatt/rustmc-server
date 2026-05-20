pub mod handshake {
    pub mod serverbound {
        pub const HANDSHAKE: i32 = 0x00;
    }
}

pub mod status {
    pub mod clientbound {
        pub const STATUS_RESPONSE: i32 = 0x00;
        pub const PONG_RESPONSE: i32 = 0x01;
    }
    pub mod serverbound {
        pub const STATUS_REQUEST: i32 = 0x00;
        pub const PING_REQUEST: i32 = 0x01;
    }
}

pub mod login {
    pub mod clientbound {
        pub const LOGIN_SUCCESS: i32 = 0x02;
        pub const SET_COMPRESSION: i32 = 0x03;
        pub const COOKIE_REQUEST: i32 = 0x05;
    }
    pub mod serverbound {
        pub const LOGIN_START: i32 = 0x00;
        pub const LOGIN_ACKNOWLEDGED: i32 = 0x03;
        pub const COOKIE_RESPONSE: i32 = 0x04;
    }
}

pub mod configuration {
    pub mod clientbound {
        pub const COOKIE_REQUEST: i32 = 0x00;
        pub const FINISH_CONFIGURATION: i32 = 0x03;
        pub const REGISTRY_DATA: i32 = 0x07;
        pub const STORE_COOKIE: i32 = 0x0A;
        pub const UPDATE_TAGS: i32 = 0x0D;
        pub const KNOWN_PACKS: i32 = 0x0E;
    }
    pub mod serverbound {
        pub const COOKIE_RESPONSE: i32 = 0x01;
        pub const ACKNOWLEDGE_FINISH: i32 = 0x03;
        pub const KNOWN_PACKS: i32 = 0x07;
    }
}

pub mod play {
    pub mod clientbound {
        pub const CHUNK_BATCH_FINISHED: i32 = 0x0B;
        pub const CHUNK_BATCH_START: i32 = 0x0C;
        pub const COOKIE_REQUEST: i32 = 0x15;
        pub const UNLOAD_CHUNK: i32 = 0x25;
        pub const GAME_EVENT: i32 = 0x26;
        pub const KEEP_ALIVE: i32 = 0x2C;
        pub const LEVEL_CHUNK_WITH_LIGHT: i32 = 0x2D;
        pub const LOGIN_PLAY: i32 = 0x31;
        pub const PLAYER_INFO_UPDATE: i32 = 0x46;
        pub const SYNCHRONIZE_PLAYER_POSITION: i32 = 0x48;
        pub const SET_CENTER_CHUNK: i32 = 0x5E;
        pub const STORE_COOKIE: i32 = 0x78;
        pub const SYSTEM_CHAT_MESSAGE: i32 = 0x79;
        pub const TRANSFER: i32 = 0x81;
    }
    pub mod serverbound {
        pub const CONFIRM_TELEPORTATION: i32 = 0x00;
        pub const CHAT_COMMAND: i32 = 0x07;
        pub const CHAT_MESSAGE: i32 = 0x09;
        pub const CHUNK_BATCH_RECEIVED: i32 = 0x0B;
        pub const CLIENT_TICK_END: i32 = 0x0D;
        pub const COOKIE_RESPONSE: i32 = 0x15;
        pub const KEEP_ALIVE: i32 = 0x1C;
        pub const SET_PLAYER_POSITION: i32 = 0x1E;
        pub const SET_PLAYER_POSITION_AND_ROTATION: i32 = 0x1F;
        pub const SET_PLAYER_ROTATION: i32 = 0x20;
        pub const SET_PLAYER_STATUS_ONLY: i32 = 0x21;
        pub const PLAYER_COMMAND: i32 = 0x2A;
        pub const PLAYER_LOADED: i32 = 0x2C;
        pub const SET_CARRIED_ITEM: i32 = 0x35;
        pub const SWING: i32 = 0x3F;
    }
}
