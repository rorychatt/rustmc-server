import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CONFIG_SCHEMA } from "../data/configOptions";
import { FileCode, Copy, Check, RotateCcw, Wrench } from "lucide-react";

export default function ConfigGenerator() {
  const { t } = useTranslation();
  const [configState, setConfigState] = useState<Record<string, any>>(() => {
    const initialState: Record<string, any> = {};
    CONFIG_SCHEMA.forEach((group) => {
      group.options.forEach((opt) => {
        initialState[opt.key] = opt.defaultValue;
      });
    });
    return initialState;
  });

  const [copied, setCopied] = useState(false);

  const handleValueChange = (key: string, value: any) => {
    setConfigState((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const resetToDefaults = () => {
    const defaultState: Record<string, any> = {};
    CONFIG_SCHEMA.forEach((group) => {
      group.options.forEach((opt) => {
        defaultState[opt.key] = opt.defaultValue;
      });
    });
    return initialState;
  };

  // Convert state into YAML format string
  const generateYaml = (): string => {
    return `# RustMC Configuration File (server.yaml)
# Configured dynamically via RustMC Interactive docs

server:
  bind: "${configState["bind"]}"
  view_distance: ${configState["view_distance"]}

rate_limit:
  invalid_packet_threshold: ${configState["invalid_packet_threshold"]}
  invalid_packet_window_secs: ${configState["invalid_packet_window_secs"]}

gameplay:
  motd: "${configState["motd"]}"
  max_players: ${configState["max_players"]}
  gamemode: "${configState["gamemode"]}"
  difficulty: "${configState["difficulty"]}"
  pvp: ${configState["pvp"]}
  allow_flight: ${configState["allow_flight"]}
  hardcore: ${configState["hardcore"]}
  simulation_distance: ${configState["simulation_distance"]}
  sea_level: ${configState["sea_level"]}
  world_type: "${configState["world_type"]}"
  seed: ${configState["seed"]}
  world_dir: "${configState["world_dir"]}"
  save_interval_secs: ${configState["save_interval_secs"]}
  backup_interval_secs: ${configState["backup_interval_secs"]}
  max_backups: ${configState["max_backups"]}
`;
  };

  const copyToClipboard = () => {
    navigator.clipboard.writeText(generateYaml());
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const initialState = (() => {
    const defaultState: Record<string, any> = {};
    CONFIG_SCHEMA.forEach((group) => {
      group.options.forEach((opt) => {
        defaultState[opt.key] = opt.defaultValue;
      });
    });
    return defaultState;
  })();

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-3xl font-bold tracking-tight text-white flex items-center gap-2">
          <Wrench className="text-emerald-400 w-8 h-8" />
          {t("config_gen.title")}
        </h2>
        <p className="text-slate-400 mt-1 text-sm">{t("config_gen.desc")}</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Configuration Controls (Left) */}
        <div className="lg:col-span-7 space-y-6">
          {CONFIG_SCHEMA.map((group) => {
            const groupTitle = t(`config.groups.${group.section}.title`, group.section);
            const groupDesc = t(`config.groups.${group.section}.description`, group.description);

            return (
              <div key={group.section} className="p-5 rounded-xl glass-panel space-y-4">
                <div>
                  <h3 className="text-lg font-bold text-white uppercase tracking-wider text-xs text-cyan-400 flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
                    {groupTitle} {t("config_gen.section_suffix")}
                  </h3>
                  <p className="text-xs text-slate-500 mt-0.5">{groupDesc}</p>
                </div>

                <div className="space-y-4 pt-2 border-t border-slate-800/40">
                  {group.options.map((opt) => {
                    const optName = t(`config.options.${opt.key}.name`, opt.name);
                    const optDesc = t(`config.options.${opt.key}.description`, opt.description);

                    return (
                      <div key={opt.key} className="flex flex-col gap-1">
                        <div className="flex items-center justify-between">
                          <label className="text-sm font-medium text-slate-300 font-mono">
                            {optName}
                          </label>
                          <span className="text-xs text-slate-500 font-mono bg-slate-950/40 px-1.5 py-0.5 rounded border border-slate-800/60">
                            {opt.key}
                          </span>
                        </div>

                        {/* Input Controls matching Option Type */}
                        {opt.type === "string" && (
                          <input
                            type="text"
                            className="w-full px-3 py-2 bg-slate-950/60 border border-slate-800 rounded-lg text-sm text-slate-300 placeholder-slate-600 focus:outline-none focus:border-cyan-500/60 focus:ring-1 focus:ring-cyan-500/20"
                            value={configState[opt.key]}
                            onChange={(e) => handleValueChange(opt.key, e.target.value)}
                          />
                        )}

                        {opt.type === "number" && (
                          <div className="flex items-center gap-3">
                            {opt.min !== undefined && opt.max !== undefined ? (
                              <>
                                <input
                                  type="range"
                                  min={opt.min}
                                  max={opt.max}
                                  className="flex-grow h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-cyan-400"
                                  value={configState[opt.key]}
                                  onChange={(e) =>
                                    handleValueChange(opt.key, parseInt(e.target.value))
                                  }
                                />
                                <input
                                  type="number"
                                  min={opt.min}
                                  max={opt.max}
                                  className="w-16 px-2 py-1 bg-slate-950/60 border border-slate-800 rounded text-center text-xs font-mono text-cyan-400 focus:outline-none focus:border-cyan-500/60"
                                  value={configState[opt.key]}
                                  onChange={(e) =>
                                    handleValueChange(
                                      opt.key,
                                      parseInt(e.target.value) || opt.defaultValue,
                                    )
                                  }
                                />
                              </>
                            ) : (
                              <input
                                type="number"
                                className="w-full px-3 py-2 bg-slate-950/60 border border-slate-800 rounded-lg text-sm text-slate-300 placeholder-slate-600 focus:outline-none focus:border-cyan-500/60 focus:ring-1 focus:ring-cyan-500/20"
                                value={configState[opt.key]}
                                onChange={(e) =>
                                  handleValueChange(opt.key, parseInt(e.target.value) || 0)
                                }
                              />
                            )}
                          </div>
                        )}

                        {opt.type === "boolean" && (
                          <div className="flex items-center mt-1">
                            <button
                              onClick={() => handleValueChange(opt.key, !configState[opt.key])}
                              className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                                configState[opt.key] ? "bg-emerald-500" : "bg-slate-800"
                              }`}
                            >
                              <span
                                className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                                  configState[opt.key] ? "translate-x-5" : "translate-x-0"
                                }`}
                              />
                            </button>
                            <span className="ml-3 text-xs text-slate-400 font-mono">
                              {configState[opt.key] ? "true" : "false"}
                            </span>
                          </div>
                        )}

                        {opt.type === "select" && (
                          <select
                            className="w-full px-3 py-2 bg-slate-950/60 border border-slate-800 rounded-lg text-sm text-slate-300 focus:outline-none focus:border-cyan-500/60"
                            value={configState[opt.key]}
                            onChange={(e) => handleValueChange(opt.key, e.target.value)}
                          >
                            {opt.options?.map((option) => (
                              <option key={option} value={option}>
                                {option}
                              </option>
                            ))}
                          </select>
                        )}

                        <p className="text-xs text-slate-400 mt-1 leading-relaxed">{optDesc}</p>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}

          {/* Reset button */}
          <div className="flex justify-end">
            <button
              onClick={resetToDefaults}
              className="px-4 py-2 text-xs font-semibold text-slate-400 hover:text-white flex items-center gap-1.5 rounded-lg border border-slate-800 hover:bg-slate-900/40 transition-colors focus:outline-none"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              {t("config_gen.reset")}
            </button>
          </div>
        </div>

        {/* YAML Preview Output (Right) */}
        <div className="lg:col-span-5 flex flex-col h-full min-h-[500px] lg:sticky lg:top-6">
          <div className="flex-grow flex flex-col rounded-xl glass-panel overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between px-4 py-3 bg-slate-950/70 border-b border-slate-800/80">
              <span className="flex items-center gap-2 text-xs font-mono text-slate-400">
                <FileCode className="text-emerald-400 w-4 h-4" />
                server.yaml
              </span>
              <button
                onClick={copyToClipboard}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-semibold focus:outline-none transition-colors ${
                  copied
                    ? "bg-emerald-500/10 text-emerald-400 border border-emerald-500/30"
                    : "bg-cyan-500/10 text-cyan-400 border border-cyan-500/30 hover:bg-cyan-500/20"
                }`}
              >
                {copied ? (
                  <>
                    <Check className="w-3.5 h-3.5" />
                    {t("config_gen.copied")}
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    {t("config_gen.copy")}
                  </>
                )}
              </button>
            </div>

            {/* Code Content */}
            <div className="flex-grow bg-slate-950/50 p-5 font-mono text-sm overflow-auto text-emerald-400/90 leading-relaxed border-none">
              <pre className="whitespace-pre">{generateYaml()}</pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
