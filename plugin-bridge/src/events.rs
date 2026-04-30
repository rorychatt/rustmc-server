use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventPriority {
    Lowest = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
    Monitor = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    PlayerJoin {
        uuid: String,
        name: String,
    },
    PlayerLeave {
        uuid: String,
        name: String,
    },
    PlayerChat {
        uuid: String,
        name: String,
        message: String,
    },
    BlockBreak {
        uuid: String,
        x: i32,
        y: i32,
        z: i32,
    },
    BlockPlace {
        uuid: String,
        x: i32,
        y: i32,
        z: i32,
        block_type: String,
    },
    ServerTick {
        tick_number: u64,
    },
    Custom {
        event_type: String,
        data: String,
    },
}

impl Event {
    pub fn event_type(&self) -> &str {
        match self {
            Event::PlayerJoin { .. } => "PlayerJoinEvent",
            Event::PlayerLeave { .. } => "PlayerLeaveEvent",
            Event::PlayerChat { .. } => "PlayerChatEvent",
            Event::BlockBreak { .. } => "BlockBreakEvent",
            Event::BlockPlace { .. } => "BlockPlaceEvent",
            Event::ServerTick { .. } => "ServerTickEvent",
            Event::Custom { event_type, .. } => event_type,
        }
    }
}

pub type EventHandler = Arc<dyn Fn(&Event) -> bool + Send + Sync>;

struct HandlerEntry {
    priority: EventPriority,
    handler: EventHandler,
    plugin_id: String,
}

pub struct EventBus {
    handlers: Mutex<HashMap<String, Vec<HandlerEntry>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(
        &self,
        event_type: &str,
        plugin_id: &str,
        priority: EventPriority,
        handler: EventHandler,
    ) {
        let mut handlers = self.handlers.lock().unwrap();
        let entries = handlers.entry(event_type.to_string()).or_default();
        entries.push(HandlerEntry {
            priority,
            handler,
            plugin_id: plugin_id.to_string(),
        });
        entries.sort_by_key(|e| e.priority);
    }

    pub fn fire(&self, event: &Event) -> bool {
        let handlers = self.handlers.lock().unwrap();
        if let Some(entries) = handlers.get(event.event_type()) {
            for entry in entries {
                let cancelled = (entry.handler)(event);
                if cancelled && entry.priority != EventPriority::Monitor {
                    return true;
                }
            }
        }
        false
    }

    pub fn unregister_plugin(&self, plugin_id: &str) {
        let mut handlers = self.handlers.lock().unwrap();
        for entries in handlers.values_mut() {
            entries.retain(|e| e.plugin_id != plugin_id);
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_fire_and_handle() {
        let bus = EventBus::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        bus.register(
            "PlayerJoinEvent",
            "test-plugin",
            EventPriority::Normal,
            Arc::new(move |_event| {
                *called_clone.lock().unwrap() = true;
                false
            }),
        );

        let event = Event::PlayerJoin {
            uuid: "test-uuid".to_string(),
            name: "TestPlayer".to_string(),
        };
        bus.fire(&event);
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_event_cancellation() {
        let bus = EventBus::new();

        bus.register(
            "PlayerChatEvent",
            "filter-plugin",
            EventPriority::High,
            Arc::new(|_event| true), // Cancel
        );

        let event = Event::PlayerChat {
            uuid: "uuid".to_string(),
            name: "Player".to_string(),
            message: "test".to_string(),
        };
        assert!(bus.fire(&event)); // Should be cancelled
    }

    #[test]
    fn test_unregister_plugin() {
        let bus = EventBus::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        bus.register(
            "PlayerJoinEvent",
            "removable",
            EventPriority::Normal,
            Arc::new(move |_| {
                *called_clone.lock().unwrap() = true;
                false
            }),
        );

        bus.unregister_plugin("removable");

        let event = Event::PlayerJoin {
            uuid: "uuid".to_string(),
            name: "Player".to_string(),
        };
        bus.fire(&event);
        assert!(!*called.lock().unwrap()); // Should NOT have been called
    }
}
