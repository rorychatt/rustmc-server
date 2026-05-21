import { useState } from 'react';
import PacketReference from './components/PacketReference';
import ConfigGenerator from './components/ConfigGenerator';
import { 
  BookOpen, 
  Cpu, 
  Settings, 
  GitFork, 
  Key, 
  ShieldAlert, 
  ExternalLink, 
  Terminal, 
  Menu, 
  X, 
  ArrowRight,
  CheckCircle,
  FileText,
  Globe,
  Database
} from 'lucide-react';

type TabId = 'overview' | 'architecture' | 'world' | 'config' | 'protocol' | 'transfer' | 'security';

export default function App() {
  const [activeTab, setActiveTab] = useState<TabId>('overview');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const tabs = [
    { id: 'overview', name: 'Overview', icon: BookOpen },
    { id: 'architecture', name: 'Server Architecture', icon: Cpu },
    { id: 'world', name: 'World & Persistence', icon: Globe },
    { id: 'config', name: 'Config Generator', icon: Settings },
    { id: 'protocol', name: 'Protocol & Packets', icon: GitFork },
    { id: 'transfer', name: 'Server Transfer', icon: Key },
    { id: 'security', name: 'Security & Ops', icon: ShieldAlert },
  ] as const;

  const handleTabChange = (tabId: TabId) => {
    setActiveTab(tabId);
    setMobileMenuOpen(false);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  return (
    <div className="min-h-screen bg-[#080d16] text-slate-100 flex flex-col relative">
      {/* Background glow spots */}
      <div className="glow-spot top-[-100px] left-[-100px] opacity-40"></div>
      <div className="glow-spot bottom-[-200px] right-[-100px] opacity-30"></div>

      {/* Top Header Navigation */}
      <header className="sticky top-0 z-40 bg-slate-950/75 backdrop-blur-md border-b border-slate-800/80 px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-400 to-emerald-500 flex items-center justify-center font-bold text-slate-950 text-xl border border-cyan-300/30 shadow-lg shadow-cyan-500/10">
            🦀
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight text-white flex items-center gap-2">
              RustMC <span className="text-xs bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 px-2 py-0.5 rounded font-mono font-bold">v26.1.2</span>
            </h1>
            <p className="text-[10px] text-slate-400 font-mono">Minecraft 1.20.5+ Protocol v775</p>
          </div>
        </div>

        {/* Desktop Links */}
        <div className="hidden md:flex items-center gap-5">
          <a
            href="https://github.com/rorychatt/rustmc-server"
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs font-semibold text-slate-300 hover:text-white flex items-center gap-1 bg-slate-900/60 border border-slate-800 px-3 py-1.5 rounded-lg hover:bg-slate-800/60 transition-all"
          >
            GitHub Repository
            <ExternalLink className="w-3.5 h-3.5" />
          </a>
        </div>

        {/* Mobile menu trigger */}
        <button
          onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          className="md:hidden p-2 text-slate-400 hover:text-white focus:outline-none"
        >
          {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
        </button>
      </header>

      {/* Mobile Drawer menu */}
      {mobileMenuOpen && (
        <div className="md:hidden fixed inset-0 z-30 bg-slate-950/95 backdrop-blur-lg pt-24 px-6 space-y-4">
          <div className="text-xs uppercase tracking-wider text-slate-500 font-bold mb-2">Documentation Sections</div>
          {tabs.map(tab => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => handleTabChange(tab.id)}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl border text-sm font-semibold transition-all ${
                  isActive
                    ? 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30'
                    : 'bg-slate-900/40 text-slate-400 border-slate-800/60 hover:text-white'
                }`}
              >
                <Icon className="w-5 h-5" />
                {tab.name}
              </button>
            );
          })}
        </div>
      )}

      {/* Main Container Layout */}
      <div className="flex-grow max-w-7xl w-full mx-auto px-6 py-8 grid grid-cols-1 md:grid-cols-12 gap-8 items-start relative z-10">
        
        {/* Sticky Left Sidebar Navigation */}
        <aside className="hidden md:block md:col-span-3 sticky top-24 space-y-2">
          <div className="text-[10px] uppercase font-bold text-slate-500 tracking-widest pl-3 mb-3">Navigation</div>
          {tabs.map(tab => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => handleTabChange(tab.id)}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl border text-sm font-semibold transition-all text-left ${
                  isActive
                    ? 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30 border-glow-cyan'
                    : 'bg-slate-900/30 text-slate-400 border-transparent hover:text-slate-200 hover:bg-slate-900/50'
                }`}
              >
                <Icon className="w-4 h-4" />
                {tab.name}
              </button>
            );
          })}
        </aside>

        {/* Content Display Panels (Right) */}
        <main className="md:col-span-9 space-y-8">
          
          {/* Tab 1: Overview */}
          {activeTab === 'overview' && (
            <div className="space-y-8 animate-fadeIn">
              {/* Hero Banner card */}
              <div className="relative rounded-2xl overflow-hidden border border-slate-800/80 bg-gradient-to-br from-slate-900 via-slate-950 to-slate-900 p-8 md:p-10 shadow-2xl">
                <div className="absolute top-0 right-0 w-64 h-64 bg-cyan-500/5 rounded-full filter blur-3xl pointer-events-none"></div>
                <div className="max-w-2xl space-y-4">
                  <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    <CheckCircle className="w-3.5 h-3.5" /> High Performance Rust Core
                  </span>
                  <h2 className="text-3xl md:text-4xl font-extrabold text-white tracking-tight leading-tight">
                    Custom Lightweight Minecraft Server In <span className="text-gradient-cyan">Rust</span>
                  </h2>
                  <p className="text-slate-400 text-sm md:text-base leading-relaxed">
                    RustMC is a custom, highly modular, thread-safe Minecraft server built from the ground up in Rust. Designed to support Minecraft 1.20.5+ (Protocol 775) status querying, handshakes, login compression, configuration syncing, and flat world multi-player sandbox interaction.
                  </p>
                  <div className="flex flex-wrap gap-4 pt-2">
                    <button
                      onClick={() => handleTabChange('architecture')}
                      className="px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-cyan-600 hover:from-cyan-400 hover:to-cyan-500 text-slate-950 font-bold text-sm rounded-lg shadow-lg shadow-cyan-500/10 transition-all flex items-center gap-1.5 focus:outline-none"
                    >
                      Explore Server Architecture
                      <ArrowRight className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => handleTabChange('config')}
                      className="px-5 py-2.5 bg-slate-900 border border-slate-800 hover:border-slate-700 font-semibold text-slate-200 text-sm rounded-lg hover:bg-slate-850 transition-all flex items-center gap-1.5 focus:outline-none"
                    >
                      Generate configuration
                    </button>
                  </div>
                </div>
              </div>

              {/* Core Features Grid */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
                <div className="p-6 rounded-xl glass-panel glass-panel-hover space-y-2">
                  <div className="w-10 h-10 rounded-lg bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 flex items-center justify-center mb-4">
                    <Cpu className="w-5 h-5" />
                  </div>
                  <h3 className="text-base font-bold text-white">Tokio Concurrency</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Async multi-threading loops handle hundreds of concurrent player handshakes, tick cycles, and chunk streaming without frame drops.
                  </p>
                </div>

                <div className="p-6 rounded-xl glass-panel glass-panel-hover space-y-2">
                  <div className="w-10 h-10 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center justify-center mb-4">
                    <Settings className="w-5 h-5" />
                  </div>
                  <h3 className="text-base font-bold text-white">Dynamic YAML Configuration</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Fully decoupled configuration parsing (similar to Paper/Spigot settings) dynamically maps MOTD, game mode, max players, pvp, flight flags, and limits.
                  </p>
                </div>

                <div className="p-6 rounded-xl glass-panel glass-panel-hover space-y-2">
                  <div className="w-10 h-10 rounded-lg bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 flex items-center justify-center mb-4">
                    <Key className="w-5 h-5" />
                  </div>
                  <h3 className="text-base font-bold text-white">Cross-Server Transfers</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Implements secure Cookie Transfer negotiation to authenticate and hand off player sessions between federated servers smoothly.
                  </p>
                </div>
              </div>

              {/* Getting Started Guide */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Terminal className="text-emerald-400 w-5 h-5" />
                  Quick Start Guide
                </h3>
                <p className="text-slate-400 text-sm">
                  Build and run the RustMC server binary locally. Follow the steps below:
                </p>
                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-emerald-400/90 space-y-2 overflow-x-auto">
                  <div><span className="text-slate-500"># Clone the repository</span></div>
                  <div>git clone https://github.com/rorychatt/rustmc-server.git</div>
                  <div>cd rustmc-server</div>
                  <div><span className="text-slate-500"># Build the workspace in release mode</span></div>
                  <div>cargo build --release</div>
                  <div><span className="text-slate-500"># Run the server. It will auto-create a default server.yaml configuration</span></div>
                  <div>./target/release/server</div>
                </div>
              </div>
            </div>
          )}

          {/* Tab 2: Architecture */}
          {activeTab === 'architecture' && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <Cpu className="text-cyan-400 w-6 h-6" />
                  Core Server Architecture
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">
                  RustMC implements a highly concurrent network handler structured around Tokio tasks, actor channels, and shared synchronized memory regions.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Loop splitting */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
                    Connection Reader & Writer Splitting
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Upon accepting a TCP client socket, RustMC splits the stream into a <strong>Reader</strong> loop and a <strong>Writer</strong> loop. This eliminates read/write thread contention. The Reader loop decodes packets, processes clientbound commands, and communicates state changes to a centralized loop via Tokio channels, while the Writer actor flushes outbound packet buffers.
                  </p>
                </div>

                {/* World State */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    In-Memory Shared World State
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    World layouts, player metadata registries, and blocks configuration are managed in memory, wrapped in thread-safe shared references: <code>Arc&lt;RwLock&lt;WorldState&gt;&gt;</code>. Multiple reader threads can retrieve player position changes simultaneously, while write locks are only requested during tick updates, ensuring minimal mutex contention.
                  </p>
                </div>
              </div>

              {/* Chat broadcast and Operators details */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white">Event Broadcasting</h3>
                <p className="text-slate-400 text-sm leading-relaxed">
                  The server utilizes a centralized <code>tokio::sync::broadcast</code> channel to propagate text chat, joint alerts, server notices, and player movement synchronization updates to all connection loops.
                </p>
                <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800 text-xs space-y-2">
                  <div className="font-mono text-cyan-400">Connection Actor loop flow:</div>
                  <ol className="list-decimal list-inside text-slate-400 space-y-1 font-mono text-[11px]">
                    <li>Accepts TCP socket connection.</li>
                    <li>Launches connection worker actor task.</li>
                    <li>Awaits handshake packet to parse next stage (Status or Login).</li>
                    <li>Authenticates player & configures compression.</li>
                    <li>Transitions client connection into Play phase.</li>
                    <li>Subscribes to global server broadcast channel.</li>
                  </ol>
                </div>
              </div>
            </div>
          )}

          {/* Tab: World & Persistence */}
          {activeTab === 'world' && (
            <div className="space-y-6 animate-fadeIn">
              {/* Header Card */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <Globe className="text-cyan-400 w-6 h-6" />
                  World Generation & Chunk Persistence
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">
                  RustMC implements a high-performance world orchestration system that handles custom world generation, dynamic spawn height detection, asynchronous chunk serialization via Zlib compression, and robust automated backups.
                </p>
              </div>

              {/* World Generation Section */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="p-6 rounded-xl glass-panel space-y-4">
                  <h3 className="text-lg font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
                    World Generators
                  </h3>
                  <div className="space-y-3 text-slate-400 text-xs leading-relaxed">
                    <p>
                      The server supports two world generation styles, configurable via the <code>world_type</code> parameter in <code>server.yaml</code>:
                    </p>
                    <ul className="list-disc list-inside space-y-2 pl-2">
                      <li>
                        <strong className="text-white">Flat World (<code>flat</code>):</strong> Generates a uniform layered terrain consisting of 1 Bedrock, 2 Dirt, and 1 Grass block at the configured <code>sea_level</code> (default Y=63). Ideal for testing and lightweight lobby setups.
                      </li>
                      <li>
                        <strong className="text-white">Normal World (<code>normal</code>):</strong> Utilizes a noise generator based on a pseudo-random number seed (specified by <code>seed</code>) to produce natural terrain contours, hills, and valleys that emulate standard Minecraft environments.
                      </li>
                    </ul>
                  </div>
                </div>

                <div className="p-6 rounded-xl glass-panel space-y-4">
                  <h3 className="text-lg font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    Spawn Height Detection
                  </h3>
                  <div className="space-y-3 text-slate-400 text-xs leading-relaxed">
                    <p>
                      To prevent players from suffocating or falling endlessly into the void when spawning, RustMC implements an automated <strong>Spawn Height Finder</strong>:
                    </p>
                    <ol className="list-decimal list-inside space-y-2 pl-2 font-mono text-[11px]">
                      <li>Computes the spawn chunk coordinate.</li>
                      <li>Scans downward from the sky limit (Y=319) at the target (X, Z).</li>
                      <li>Locates the first non-air block state.</li>
                      <li>Ensures the block is solid (not air or fluid).</li>
                      <li>Sets spawn position precisely on top of this block.</li>
                    </ol>
                  </div>
                </div>
              </div>

              {/* Persistence Details */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Database className="text-cyan-400 w-5 h-5" />
                  Chunk Serialization & Compression
                </h3>
                <p className="text-slate-400 text-sm leading-relaxed">
                  World chunks are persisted to disk inside the directory configured by <code>world_dir</code>. Each chunk is saved in its own file using a compressed JSON strategy to maximize space efficiency:
                </p>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-4 pt-2">
                  <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800/60 text-xs space-y-2">
                    <div className="font-bold text-white font-mono">1. Metadata (level.json)</div>
                    <p className="text-slate-400 leading-normal">
                      Saves world-wide attributes including spawn coordinates (X, Y, Z), world seed, world type, difficulty, and sea level.
                    </p>
                  </div>
                  <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800/60 text-xs space-y-2">
                    <div className="font-bold text-white font-mono">2. Chunk Files (c.X.Z.json.zlib)</div>
                    <p className="text-slate-400 leading-normal">
                      Individual chunk columns are serialized to JSON structure containing block state palettes, non-air block counts, and block arrays, then compressed using Zlib.
                    </p>
                  </div>
                  <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800/60 text-xs space-y-2">
                    <div className="font-bold text-white font-mono">3. Async File I/O</div>
                    <p className="text-slate-400 leading-normal">
                      All reading and writing of compressed chunk data is performed using Rust's file system handles, ensuring integrity and speed.
                    </p>
                  </div>
                </div>

                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-emerald-400/90 space-y-1 overflow-x-auto">
                  <div className="text-slate-500">// Directory layout on disk:</div>
                  <div>world/</div>
                  <div>├── level.json                 <span className="text-slate-500"># World configuration metadata</span></div>
                  <div>└── chunks/                   <span className="text-slate-500"># Compressed chunk files</span></div>
                  <div>    ├── c.0.0.json.zlib        <span className="text-slate-500"># Chunk at (0, 0)</span></div>
                  <div>    ├── c.0.1.json.zlib        <span className="text-slate-500"># Chunk at (0, 1)</span></div>
                  <div>    └── c.-1.0.json.zlib       <span className="text-slate-500"># Chunk at (-1, 0)</span></div>
                </div>
              </div>

              {/* Background Loops & Backups */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Terminal className="text-emerald-400 w-5 h-5" />
                  Automated Background Tasks & Backups
                </h3>
                <p className="text-slate-400 text-sm leading-relaxed">
                  RustMC manages persistence operations in the background using lightweight asynchronous Tokio loops, keeping the server main tick thread responsive:
                </p>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div className="p-5 rounded-lg bg-slate-950/40 border border-slate-800/60 space-y-3">
                    <h4 className="text-sm font-bold text-white">Periodic Autosave Loop</h4>
                    <p className="text-slate-400 text-xs leading-relaxed">
                      Runs in a background task at the frequency set by <code>save_interval_secs</code> (default: 300s). The loop acquires a read lock on the world state, identifies modified/dirty chunks, and serializes them asynchronously to avoid interrupting connection processing.
                    </p>
                  </div>
                  
                  <div className="p-5 rounded-lg bg-slate-950/40 border border-slate-800/60 space-y-3">
                    <h4 className="text-sm font-bold text-white">Automated Backups & Rotation</h4>
                    <p className="text-slate-400 text-xs leading-relaxed">
                      Runs at the frequency set by <code>backup_interval_secs</code> (default: 3600s). The task creates a full copy of the world directory under <code>backups/backup_&lt;millisecond_timestamp&gt;/</code>. To prevent blocking the async runtime, I/O copying is offloaded to a thread pool via <code>tokio::task::spawn_blocking</code>.
                    </p>
                  </div>
                </div>

                <div className="p-5 rounded-lg bg-slate-950/40 border border-slate-800/60 space-y-3">
                  <h4 className="text-sm font-bold text-white">Backup Pruning Policy</h4>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    To prevent the storage volume from filling up over time, the backup system enforces a pruning algorithm: after a successful backup, it scans the backups directory, sorts backups chronologically, and deletes the oldest folders so that only the latest <code>max_backups</code> (default: 5) are kept.
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Tab 3: Config Generator */}
          {activeTab === 'config' && <ConfigGenerator />}

          {/* Tab 4: Protocol & Packets */}
          {activeTab === 'protocol' && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-3">
                <h2 className="text-2xl font-bold text-white">Minecraft Protocol Flow</h2>
                <p className="text-slate-400 text-sm leading-relaxed">
                  Connections traverse a sequential multi-state workflow defined by the Minecraft networking protocol specification.
                </p>
                
                {/* Visual diagram representation */}
                <div className="flex flex-col md:flex-row justify-between items-center gap-4 py-6 px-4 rounded-lg bg-slate-950/40 border border-slate-800 text-xs font-mono text-center">
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-cyan-500/20 bg-cyan-950/10 text-cyan-300">
                    <span className="block font-bold">1. Handshake</span>
                    <span className="text-[10px] text-slate-500">Port, Serverbound</span>
                  </div>
                  <div className="text-slate-500">➔</div>
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-emerald-500/20 bg-emerald-950/10 text-emerald-300">
                    <span className="block font-bold">2. Status / Ping</span>
                    <span className="text-[10px] text-slate-500">Query & Latency</span>
                  </div>
                  <div className="text-slate-500">OR ➔</div>
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-cyan-500/20 bg-cyan-950/10 text-cyan-300">
                    <span className="block font-bold">3. Login</span>
                    <span className="text-[10px] text-slate-500">Auth & Compression</span>
                  </div>
                  <div className="text-slate-500">➔</div>
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-cyan-500/20 bg-cyan-950/10 text-cyan-300">
                    <span className="block font-bold">4. Play Phase</span>
                    <span className="text-[10px] text-slate-500">Terrain Ticks & Chat</span>
                  </div>
                </div>
              </div>

              <PacketReference />
            </div>
          )}

          {/* Tab 5: Transfer Protocol */}
          {activeTab === 'transfer' && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <Key className="text-cyan-400 w-6 h-6" />
                  Cross-Server Cookie Transfer Protocol
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">
                  RustMC implements the modern Minecraft cookie transfer and redirection handshake to allow seamless hub-to-spoke network redirects.
                </p>
              </div>

              {/* Steps grid */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
                <div className="p-5 rounded-xl glass-panel space-y-2">
                  <span className="text-lg font-mono text-cyan-400 font-bold">01</span>
                  <h3 className="text-sm font-bold text-white">Negotiate Transfer</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    The source server sends a Transfer packet to the client, supplying the destination server hostname and port along with unique routing parameters.
                  </p>
                </div>

                <div className="p-5 rounded-xl glass-panel space-y-2">
                  <span className="text-lg font-mono text-emerald-400 font-bold">02</span>
                  <h3 className="text-sm font-bold text-white">Store Cookie Token</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Prior to the client transitioning, the source server posts a cryptographic verification token (cookie token) to the destination server's auth cache.
                  </p>
                </div>

                <div className="p-5 rounded-xl glass-panel space-y-2">
                  <span className="text-lg font-mono text-cyan-400 font-bold">03</span>
                  <h3 className="text-sm font-bold text-white">Handoff Validation</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Upon connecting to the destination server, the client transmits the verification token during the login sequence. The destination verifies it to restore state.
                  </p>
                </div>
              </div>

              {/* Code/Flow explanation card */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <FileText className="text-cyan-400 w-4 h-4" />
                  Cookie Validation Protocol Flow
                </h3>
                <p className="text-slate-400 text-sm">
                  The protocol secures transfers against session hijacking using HMAC signed request payloads exchanged between federated instances.
                </p>
                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-slate-300 leading-relaxed overflow-x-auto">
                  <div className="text-slate-500">// Pseudo-code logic for verification on destination server:</div>
                  <div>fn verify_cookie_handshake(player_uuid: Uuid, received_token: &amp;[u8]) -&gt; bool &#123;</div>
                  <div className="pl-4">match get_cached_transfer_cookie(&amp;player_uuid) &#123;</div>
                  <div className="pl-8">Some(cached_cookie) =&gt; &#123;</div>
                  <div className="pl-12">constant_time_compare(&amp;cached_cookie.token, received_token)</div>
                  <div className="pl-8">&#125;</div>
                  <div className="pl-8">None =&gt; false</div>
                  <div className="pl-4">&#125;</div>
                  <div>&#125;</div>
                </div>
              </div>
            </div>
          )}

          {/* Tab 6: Security & Ops */}
          {activeTab === 'security' && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <ShieldAlert className="text-emerald-400 w-6 h-6" />
                  Security & Operator Permissions
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">
                  RustMC implements rate limit protections on client streams and maps administration roles dynamically via the Operators system.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Rate Limiting */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white">Connection Rate Limiting</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    To prevent denial of service (DoS) and spamming, connection actors monitor inbound packets. If a player exceeds the configured <code>invalid_packet_threshold</code> within the window specified by <code>invalid_packet_window_secs</code>, the connection is instantly severed and blacklisted for the epoch.
                  </p>
                </div>

                {/* Operator roles */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white">Operators System (ops.toml)</h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    Server operators are configured inside <code>ops.toml</code> using their username, UUID, and integer permission levels (1-4). On connection login, the server matches credentials and streams the operator rank down to the client to activate server commands.
                  </p>
                </div>
              </div>

              {/* Ops configuration file syntax */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <Terminal className="text-cyan-400 w-4 h-4" />
                  Operators File Format (ops.toml)
                </h3>
                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-emerald-400/90 leading-relaxed overflow-x-auto">
                  <div>[[operators]]</div>
                  <div>uuid = "e36e6e2f-5069-42b7-84eb-55268c13038d"</div>
                  <div>name = "rorychatt"</div>
                  <div>level = 4</div>
                  <div>bypasses_player_limit = true</div>
                </div>
              </div>
            </div>
          )}

        </main>
      </div>

      {/* Page Footer */}
      <footer className="mt-auto bg-slate-950/60 border-t border-slate-800/80 px-6 py-6 text-center text-xs text-slate-500 z-10">
        <p>© 2026 RustMC Server Project. Licensed under MIT. High-performance Minecraft gaming in Rust.</p>
      </footer>
    </div>
  );
}
