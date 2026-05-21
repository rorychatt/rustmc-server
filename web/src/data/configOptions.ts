export interface ConfigOption {
  key: string;
  name: string;
  type: 'string' | 'number' | 'boolean' | 'select';
  defaultValue: any;
  options?: string[];
  min?: number;
  max?: number;
  description: string;
}

export interface ConfigGroup {
  section: string;
  description: string;
  options: ConfigOption[];
}

export const CONFIG_SCHEMA: ConfigGroup[] = [
  {
    section: 'server',
    description: 'Core networking and server allocation configurations.',
    options: [
      {
        key: 'bind',
        name: 'Bind Address',
        type: 'string',
        defaultValue: '0.0.0.0:25565',
        description: 'The IP address and port that the RustMC server binds to.'
      },
      {
        key: 'view_distance',
        name: 'View Distance',
        type: 'number',
        defaultValue: 8,
        min: 2,
        max: 32,
        description: 'The maximum chunk distance the server will stream to players.'
      }
    ]
  },
  {
    section: 'rate_limit',
    description: 'Connection rate limits and DDoS flood protection mechanisms.',
    options: [
      {
        key: 'invalid_packet_threshold',
        name: 'Invalid Packet Threshold',
        type: 'number',
        defaultValue: 16,
        min: 1,
        max: 100,
        description: 'Maximum number of malformed or invalid packets allowed before dropping a connection.'
      },
      {
        key: 'invalid_packet_window_secs',
        name: 'Window Duration (secs)',
        type: 'number',
        defaultValue: 10,
        min: 1,
        max: 300,
        description: 'The sliding time window in seconds during which invalid packets are accumulated.'
      }
    ]
  },
  {
    section: 'gameplay',
    description: 'Minecraft mechanics, world parameters, and rule enforcements.',
    options: [
      {
        key: 'motd',
        name: 'Message of the Day (MOTD)',
        type: 'string',
        defaultValue: 'RustMC Server - A Rust-powered Minecraft server',
        description: 'The description of the server displayed in the client server list.'
      },
      {
        key: 'max_players',
        name: 'Max Players',
        type: 'number',
        defaultValue: 20,
        min: 1,
        max: 1000,
        description: 'The maximum concurrent players allowed to join the server.'
      },
      {
        key: 'gamemode',
        name: 'Game Mode',
        type: 'select',
        defaultValue: 'creative',
        options: ['survival', 'creative', 'adventure', 'spectator'],
        description: 'Initial game mode assigned to new players joining the world.'
      },
      {
        key: 'difficulty',
        name: 'Difficulty',
        type: 'select',
        defaultValue: 'normal',
        options: ['peaceful', 'easy', 'normal', 'hard'],
        description: 'Toggles structural threat levels and damage scales.'
      },
      {
        key: 'pvp',
        name: 'PVP Enabled',
        type: 'boolean',
        defaultValue: true,
        description: 'Allows players to deal combat damage to each other.'
      },
      {
        key: 'allow_flight',
        name: 'Allow Flight',
        type: 'boolean',
        defaultValue: false,
        description: 'Enables flight capabilities in survival modes, bypassing anti-cheat flags.'
      },
      {
        key: 'hardcore',
        name: 'Hardcore Mode',
        type: 'boolean',
        defaultValue: false,
        description: 'If active, players are permanently put in spectator mode upon dying.'
      },
      {
        key: 'simulation_distance',
        name: 'Simulation Distance',
        type: 'number',
        defaultValue: 8,
        min: 2,
        max: 32,
        description: 'Ticking range of chunks surrounding the player.'
      },
      {
        key: 'sea_level',
        name: 'Sea Level',
        type: 'number',
        defaultValue: 63,
        min: 0,
        max: 255,
        description: 'Base sea height level utilized during flat chunk generation.'
      },
      {
        key: 'world_type',
        name: 'World Type',
        type: 'select',
        defaultValue: 'normal',
        options: ['normal', 'flat'],
        description: 'Determines the world generation style. "flat" generates a flat grass plain, "normal" generates a noise-based surface.'
      },
      {
        key: 'seed',
        name: 'World Seed',
        type: 'number',
        defaultValue: 0,
        description: 'Seed for random number generator utilized during normal noise terrain generation.'
      },
      {
        key: 'world_dir',
        name: 'World Directory',
        type: 'string',
        defaultValue: 'world',
        description: 'Directory path where the world state (chunks, level info) is saved and loaded.'
      },
      {
        key: 'save_interval_secs',
        name: 'Save Interval (secs)',
        type: 'number',
        defaultValue: 300,
        description: 'Time interval in seconds between automatic background saving of modified chunks.'
      },
      {
        key: 'backup_interval_secs',
        name: 'Backup Interval (secs)',
        type: 'number',
        defaultValue: 3600,
        description: 'Time interval in seconds between automatic background backups of the world directory.'
      },
      {
        key: 'max_backups',
        name: 'Max Backups',
        type: 'number',
        defaultValue: 5,
        description: 'Maximum number of automated backups to keep before older backups are pruned.'
      }
    ]
  }
];
