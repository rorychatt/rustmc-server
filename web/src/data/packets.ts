export interface PacketInfo {
  id: string;
  name: string;
  phase: 'Status' | 'Login' | 'Configuration' | 'Play';
  direction: 'Clientbound' | 'Serverbound';
  description: string;
  fields: { name: string; type: string; description: string }[];
}

export const PACKETS: PacketInfo[] = [
  // Status Phase
  {
    id: '0x00',
    name: 'Status Request',
    phase: 'Status',
    direction: 'Serverbound',
    description: 'Sent by the client immediately after handshaking to request server status metadata.',
    fields: []
  },
  {
    id: '0x00',
    name: 'Status Response',
    phase: 'Status',
    direction: 'Clientbound',
    description: 'Returns a JSON structure detailing the server version, maximum and current players, and MOTD.',
    fields: [
      { name: 'JSON Response', type: 'String', description: 'Serialized JSON string conforming to Minecraft status format.' }
    ]
  },
  {
    id: '0x01',
    name: 'Ping Request',
    phase: 'Status',
    direction: 'Serverbound',
    description: 'Sent by the client to measure latency. Contains an arbitrary payload.',
    fields: [
      { name: 'Payload', type: 'Long', description: '64-bit integer timestamp or unique payload.' }
    ]
  },
  {
    id: '0x01',
    name: 'Pong Response',
    phase: 'Status',
    direction: 'Clientbound',
    description: 'Sent by the server to respond to a Ping Request, echoing back the same payload.',
    fields: [
      { name: 'Payload', type: 'Long', description: '64-bit integer timestamp corresponding to the Ping Request.' }
    ]
  },

  // Login Phase
  {
    id: '0x00',
    name: 'Hello (Login Start)',
    phase: 'Login',
    direction: 'Serverbound',
    description: 'Initiates the login flow. Specifies username and player UUID.',
    fields: [
      { name: 'Name', type: 'String', description: 'The player\'s username (maximum 16 characters).' },
      { name: 'UUID', type: 'UUID', description: 'The player\'s unique 128-bit identifier.' }
    ]
  },
  {
    id: '0x03',
    name: 'Login Compression',
    phase: 'Login',
    direction: 'Clientbound',
    description: 'Sent by the server to enable compression for all subsequent packet communications.',
    fields: [
      { name: 'Threshold', type: 'VarInt', description: 'Packet size threshold (in bytes) above which packets are zlib-compressed.' }
    ]
  },
  {
    id: '0x02',
    name: 'Login Finished',
    phase: 'Login',
    direction: 'Clientbound',
    description: 'Confirms successful authentication and transition out of the login phase.',
    fields: [
      { name: 'UUID', type: 'UUID', description: 'The authenticated player UUID.' },
      { name: 'Username', type: 'String', description: 'The authenticated player username.' },
      { name: 'Properties Count', type: 'VarInt', description: 'Number of texture profile properties.' }
    ]
  },
  {
    id: '0x03',
    name: 'Login Acknowledged',
    phase: 'Login',
    direction: 'Serverbound',
    description: 'Acknowledges receipt of Login Finished and transitions client state to Configuration.',
    fields: []
  },

  // Configuration Phase
  {
    id: '0x0E',
    name: 'Select Known Packs',
    phase: 'Configuration',
    direction: 'Clientbound',
    description: 'Server sends list of known data/resource packs for the client to confirm sync state.',
    fields: [
      { name: 'Packs Count', type: 'VarInt', description: 'Number of declared resource/data packs.' }
    ]
  },
  {
    id: '0x07',
    name: 'Select Known Packs Response',
    phase: 'Configuration',
    direction: 'Serverbound',
    description: 'Client responds with the subset of known packs that it actually has cached.',
    fields: []
  },
  {
    id: '0x07',
    name: 'Registry Data',
    phase: 'Configuration',
    direction: 'Clientbound',
    description: 'Transfers dimension types, biomes, chat types, and damage types registries in NBT formats.',
    fields: [
      { name: 'Registry ID', type: 'String', description: 'The namespace ID of the registry (e.g. minecraft:dimension_type).' },
      { name: 'Entries Count', type: 'VarInt', description: 'Number of elements in this registry.' }
    ]
  },
  {
    id: '0x0D',
    name: 'Update Tags',
    phase: 'Configuration',
    direction: 'Clientbound',
    description: 'Sends block, item, fluid, and entity tags configuration mappings to the client.',
    fields: []
  },
  {
    id: '0x03',
    name: 'Finish Configuration',
    phase: 'Configuration',
    direction: 'Clientbound',
    description: 'Sent by the server to signal completion of the configuration sync handshake.',
    fields: []
  },
  {
    id: '0x03',
    name: 'Acknowledge Finish Configuration',
    phase: 'Configuration',
    direction: 'Serverbound',
    description: 'Client acknowledges configuration completion and transitions the connection state to Play.',
    fields: []
  },

  // Play Phase
  {
    id: '0x31',
    name: 'Login (Play)',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Initial state configuration packet entering the active gameplay world. Transmits world physics settings.',
    fields: [
      { name: 'Entity ID', type: 'Int', description: 'The player\'s runtime entity ID.' },
      { name: 'Hardcore', type: 'Boolean', description: 'Whether the server is in hardcore mode.' },
      { name: 'Max Players', type: 'VarInt', description: 'The maximum player capacity.' },
      { name: 'View Distance', type: 'VarInt', description: 'The client\'s chunk view distance.' },
      { name: 'Simulation Distance', type: 'VarInt', description: 'The server\'s chunk ticking distance.' },
      { name: 'Game Mode', type: 'Byte', description: 'The player\'s initial gamemode (Survival: 0, Creative: 1, etc.).' },
      { name: 'Sea Level', type: 'VarInt', description: 'The world sea height parameter.' }
    ]
  },
  {
    id: '0x60',
    name: 'Set Entity Metadata',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Updates entity characteristics, such as visibility, equipment, or active pose.',
    fields: []
  },
  {
    id: '0x48',
    name: 'Player Position',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Synchronizes or forces the client player\'s position and rotation coordinates.',
    fields: [
      { name: 'X / Y / Z', type: 'Double', description: 'Coordinates in world space.' },
      { name: 'Yaw / Pitch', type: 'Float', description: 'Camera angles.' }
    ]
  },
  {
    id: '0x2D',
    name: 'Level Chunk With Light',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Transmits world voxel blocks, section structures, and lighting grids to the client.',
    fields: [
      { name: 'Chunk X / Z', type: 'Int', description: 'Absolute grid coordinates of the loaded chunk.' }
    ]
  },
  {
    id: '0x0C',
    name: 'Chunk Batch Start',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Signals the start of a multi-chunk loading sequence, allowing the client to batch rendering.',
    fields: []
  },
  {
    id: '0x0B',
    name: 'Chunk Batch Finished',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Signals the termination of the current chunk loading sequence batch.',
    fields: [
      { name: 'Batch Size', type: 'VarInt', description: 'Number of chunks transferred in this batch.' }
    ]
  },
  {
    id: '0x0B',
    name: 'Chunk Batch Received',
    phase: 'Play',
    direction: 'Serverbound',
    description: 'Sent by the client to confirm rendering and receipt of the chunk batch, throttling network throughput.',
    fields: []
  },
  {
    id: '0x0D',
    name: 'Client Tick End',
    phase: 'Play',
    direction: 'Serverbound',
    description: 'Sent by the client at the end of each local tick loop to trigger server update cycles.',
    fields: []
  },
  {
    id: '0x2C',
    name: 'Player Loaded',
    phase: 'Play',
    direction: 'Serverbound',
    description: 'Client signals that the local player has finished loading terrain and is ready for ticks.',
    fields: []
  },
  {
    id: '0x79',
    name: 'System Chat',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Transmits text logs or system notices to the client chat UI.',
    fields: [
      { name: 'Message', type: 'String (JSON)', description: 'Chat message formatted in Minecraft JSON Text component format.' }
    ]
  },
  {
    id: '0x2C',
    name: 'Keep Alive (Clientbound)',
    phase: 'Play',
    direction: 'Clientbound',
    description: 'Periodic server heartbeat to ensure connection viability.',
    fields: [
      { name: 'Keep Alive ID', type: 'Long', description: 'Unique heartbeat identifier.' }
    ]
  },
  {
    id: '0x1C',
    name: 'Keep Alive (Serverbound)',
    phase: 'Play',
    direction: 'Serverbound',
    description: 'Heartbeat echo response from client validating connection vitality.',
    fields: [
      { name: 'Keep Alive ID', type: 'Long', description: 'The matching identifier.' }
    ]
  }
];
