import { useState } from 'react';
import { PACKETS } from '../data/packets';
import { Search, ArrowDownRight, ArrowUpRight, ShieldAlert, Layers } from 'lucide-react';

export default function PacketReference() {
  const [search, setSearch] = useState('');
  const [selectedPhase, setSelectedPhase] = useState<'All' | 'Status' | 'Login' | 'Configuration' | 'Play'>('All');
  const [selectedDirection, setSelectedDirection] = useState<'All' | 'Clientbound' | 'Serverbound'>('All');
  const [expandedPackets, setExpandedPackets] = useState<string[]>([]);

  const toggleExpand = (key: string) => {
    setExpandedPackets(prev =>
      prev.includes(key) ? prev.filter(k => k !== key) : [...prev, key]
    );
  };

  const filteredPackets = PACKETS.filter(packet => {
    const matchesSearch =
      packet.name.toLowerCase().includes(search.toLowerCase()) ||
      packet.id.toLowerCase().includes(search.toLowerCase()) ||
      packet.description.toLowerCase().includes(search.toLowerCase());
    const matchesPhase = selectedPhase === 'All' || packet.phase === selectedPhase;
    const matchesDirection = selectedDirection === 'All' || packet.direction === selectedDirection;
    return matchesSearch && matchesPhase && matchesDirection;
  });

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row gap-4 justify-between items-start md:items-center">
        <div>
          <h2 className="text-3xl font-bold tracking-tight text-white flex items-center gap-2">
            <Layers className="text-cyan-400 w-8 h-8" />
            Minecraft Protocol Packets
          </h2>
          <p className="text-slate-400 mt-1 text-sm">
            Interactive guide to Minecraft 1.20.5+ (Protocol 775) packet framing implemented in RustMC.
          </p>
        </div>
      </div>

      {/* Filters and Search Bar */}
      <div className="grid grid-cols-1 lg:grid-cols-4 gap-4 p-4 rounded-xl glass-panel">
        {/* Search */}
        <div className="lg:col-span-2 relative">
          <Search className="absolute left-3 top-3.5 h-4 w-4 text-slate-400" />
          <input
            type="text"
            placeholder="Search packet name, ID, fields..."
            className="w-full pl-10 pr-4 py-2.5 bg-slate-950/50 border border-slate-700/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-400/70 focus:ring-1 focus:ring-cyan-400/30 text-sm transition-all"
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
        </div>

        {/* Phase Filter */}
        <div>
          <select
            className="w-full px-3 py-2.5 bg-slate-950/50 border border-slate-700/50 rounded-lg text-slate-300 focus:outline-none focus:border-cyan-400/70 text-sm transition-all"
            value={selectedPhase}
            onChange={e => setSelectedPhase(e.target.value as any)}
          >
            <option value="All">All Phases</option>
            <option value="Status">Status</option>
            <option value="Login">Login</option>
            <option value="Configuration">Configuration</option>
            <option value="Play">Play</option>
          </select>
        </div>

        {/* Direction Filter */}
        <div>
          <select
            className="w-full px-3 py-2.5 bg-slate-950/50 border border-slate-700/50 rounded-lg text-slate-300 focus:outline-none focus:border-cyan-400/70 text-sm transition-all"
            value={selectedDirection}
            onChange={e => setSelectedDirection(e.target.value as any)}
          >
            <option value="All">All Directions</option>
            <option value="Clientbound">Clientbound (S → C)</option>
            <option value="Serverbound">Serverbound (C → S)</option>
          </select>
        </div>
      </div>

      {/* Packet Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {filteredPackets.length > 0 ? (
          filteredPackets.map(packet => {
            const packetKey = `${packet.phase}-${packet.direction}-${packet.name}`;
            const isExpanded = expandedPackets.includes(packetKey);
            const isClientbound = packet.direction === 'Clientbound';

            return (
              <div
                key={packetKey}
                className="p-5 rounded-xl glass-panel glass-panel-hover flex flex-col justify-between"
              >
                <div>
                  <div className="flex items-center justify-between gap-2 mb-3">
                    <span className="text-xs font-mono px-2 py-0.5 rounded bg-slate-900 border border-slate-800 text-slate-400 font-bold">
                      {packet.phase}
                    </span>
                    <span
                      className={`text-xs px-2.5 py-0.5 rounded-full font-medium flex items-center gap-1 ${
                        isClientbound
                          ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                          : 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/20'
                      }`}
                    >
                      {isClientbound ? (
                        <>
                          <ArrowDownRight className="w-3.5 h-3.5" />
                          Clientbound
                        </>
                      ) : (
                        <>
                          <ArrowUpRight className="w-3.5 h-3.5" />
                          Serverbound
                        </>
                      )}
                    </span>
                  </div>

                  <div className="flex items-baseline justify-between mb-2">
                    <h3 className="text-lg font-bold text-white tracking-tight">
                      {packet.name}
                    </h3>
                    <span className="text-sm font-mono text-cyan-400 font-bold">
                      {packet.id}
                    </span>
                  </div>

                  <p className="text-slate-400 text-sm leading-relaxed mb-4">
                    {packet.description}
                  </p>
                </div>

                {/* Fields Expandable section */}
                <div className="mt-2 pt-3 border-t border-slate-800/60">
                  {packet.fields.length > 0 ? (
                    <div>
                      <button
                        onClick={() => toggleExpand(packetKey)}
                        className="text-xs text-cyan-400/80 hover:text-cyan-400 flex items-center gap-1 focus:outline-none transition-colors"
                      >
                        {isExpanded ? 'Hide Payload Schema' : 'Show Payload Schema'} ({packet.fields.length})
                      </button>

                      {isExpanded && (
                        <div className="mt-3 overflow-hidden rounded-lg border border-slate-800/80 bg-slate-950/40">
                          <table className="w-full text-left text-xs border-collapse">
                            <thead>
                              <tr className="bg-slate-900/60 border-b border-slate-800 text-slate-300">
                                <th className="p-2 font-semibold">Field</th>
                                <th className="p-2 font-semibold">Type</th>
                                <th className="p-2 font-semibold">Description</th>
                              </tr>
                            </thead>
                            <tbody className="divide-y divide-slate-800/40 text-slate-400">
                              {packet.fields.map(field => (
                                <tr key={field.name} className="hover:bg-slate-900/20">
                                  <td className="p-2 font-mono text-slate-300">{field.name}</td>
                                  <td className="p-2 font-mono text-cyan-400/90">{field.type}</td>
                                  <td className="p-2">{field.description}</td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      )}
                    </div>
                  ) : (
                    <span className="text-xs text-slate-500 italic">
                      Empty payload or dynamic data structure
                    </span>
                  )}
                </div>
              </div>
            );
          })
        ) : (
          <div className="col-span-2 py-12 flex flex-col items-center justify-center rounded-xl glass-panel text-slate-500">
            <ShieldAlert className="w-10 h-10 text-slate-600 mb-2" />
            <p className="text-sm">No packets match the filter criteria.</p>
          </div>
        )}
      </div>
    </div>
  );
}
