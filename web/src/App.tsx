import { useState } from "react";
import { useTranslation } from "react-i18next";
import PacketReference from "./components/PacketReference";
import ConfigGenerator from "./components/ConfigGenerator";
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
  Database,
} from "lucide-react";

type TabId =
  | "overview"
  | "architecture"
  | "world"
  | "config"
  | "protocol"
  | "transfer"
  | "security";

// JavaScript constants to keep terminal commands, config lines, and code snippets out of JSX string-literal detection
const CLONE_CMD = "git clone https://github.com/rorychatt/rustmc-server.git";
const CD_CMD = "cd rustmc-server";
const BUILD_CMD = "cargo build --release";
const RUN_CMD = "./target/release/server";

const WORLD_DIR_NAME = "world/";
const LEVEL_JSON = "├── level.json                 ";
const CHUNKS_DIR = "└── chunks/                   ";
const CHUNK_0_0 = "    ├── c.0.0.json.zlib        ";
const CHUNK_0_1 = "    ├── c.0.1.json.zlib        ";
const CHUNK_M1_0 = "    └── c.-1.0.json.zlib       ";

const CODE_LINE_1 = "fn verify_cookie_handshake(player_uuid: Uuid, received_token: &[u8]) -> bool {";
const CODE_LINE_2 = "    match get_cached_transfer_cookie(&player_uuid) {";
const CODE_LINE_3 = "        Some(cached_cookie) => {";
const CODE_LINE_4 = "            constant_time_compare(&cached_cookie.token, received_token)";
const CODE_LINE_5 = "        }";
const CODE_LINE_6 = "        None => false";
const CODE_LINE_7 = "    }";
const CODE_LINE_8 = "}";

const OPS_HEADER = "[[operators]]";
const OPS_UUID = 'uuid = "e36e6e2f-5069-42b7-84eb-55268c13038d"';
const OPS_NAME = 'name = "rorychatt"';
const OPS_LEVEL = "level = 4";
const OPS_BYPASS = "bypasses_player_limit = true";

export default function App() {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabId>("overview");
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [langMenuOpen, setLangMenuOpen] = useState(false);

  const currentLanguage = i18n.language || "en";

  const tabs = [
    { id: "overview", name: t("nav.overview"), icon: BookOpen },
    { id: "architecture", name: t("nav.architecture"), icon: Cpu },
    { id: "world", name: t("nav.world"), icon: Globe },
    { id: "config", name: t("nav.config"), icon: Settings },
    { id: "protocol", name: t("nav.protocol"), icon: GitFork },
    { id: "transfer", name: t("nav.transfer"), icon: Key },
    { id: "security", name: t("nav.security"), icon: ShieldAlert },
  ] as const;

  const handleTabChange = (tabId: TabId) => {
    setActiveTab(tabId);
    setMobileMenuOpen(false);
    window.scrollTo({ top: 0, behavior: "smooth" });
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
              {t("header.title")}{" "}
              <span className="text-xs bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 px-2 py-0.5 rounded font-mono font-bold">
                {t("header.version")}
              </span>
            </h1>
            <p className="text-[10px] text-slate-400 font-mono">{t("header.protocol")}</p>
          </div>
        </div>

        {/* Language selector & Desktop Links */}
        <div className="hidden md:flex items-center gap-4">
          <div className="relative">
            <button
              onClick={() => setLangMenuOpen(!langMenuOpen)}
              className="text-xs font-semibold text-slate-300 hover:text-white flex items-center gap-1.5 bg-slate-900/60 border border-slate-800 px-3 py-1.5 rounded-lg hover:bg-slate-800/60 transition-all focus:outline-none"
            >
              <Globe className="w-3.5 h-3.5 text-cyan-400 animate-pulse" />
              <span>{currentLanguage.startsWith("es") ? t("languages.spanish") : t("languages.english")}</span>
              <span className="text-[10px] text-slate-500">▼</span>
            </button>
            {langMenuOpen && (
              <div className="absolute right-0 mt-2 w-32 rounded-lg bg-slate-950 border border-slate-800 shadow-xl overflow-hidden z-50 animate-fadeIn">
                <button
                  onClick={() => {
                    i18n.changeLanguage("en");
                    setLangMenuOpen(false);
                  }}
                  className={`w-full text-left px-3 py-2 text-xs hover:bg-slate-900 transition-colors ${
                    currentLanguage.startsWith("en")
                      ? "text-cyan-400 font-bold bg-slate-900/50"
                      : "text-slate-300"
                  }`}
                >
                  {t("languages.english")}
                </button>
                <button
                  onClick={() => {
                    i18n.changeLanguage("es");
                    setLangMenuOpen(false);
                  }}
                  className={`w-full text-left px-3 py-2 text-xs hover:bg-slate-900 transition-colors ${
                    currentLanguage.startsWith("es")
                      ? "text-cyan-400 font-bold bg-slate-900/50"
                      : "text-slate-300"
                  }`}
                >
                  {t("languages.spanish")}
                </button>
              </div>
            )}
          </div>

          <a
            href="https://github.com/rorychatt/rustmc-server"
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs font-semibold text-slate-300 hover:text-white flex items-center gap-1 bg-slate-900/60 border border-slate-800 px-3 py-1.5 rounded-lg hover:bg-slate-800/60 transition-all"
          >
            {t("header.github")}
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
          <div className="flex items-center justify-between border-b border-slate-800 pb-4 mb-2">
            <span className="text-xs uppercase tracking-wider text-slate-500 font-bold">
              {t("nav.sections")}
            </span>
            <div className="flex gap-2">
              <button
                onClick={() => i18n.changeLanguage("en")}
                className={`px-2 py-1 rounded text-xs border ${
                  currentLanguage.startsWith("en")
                    ? "bg-cyan-500/10 text-cyan-400 border-cyan-500/30 font-bold"
                    : "bg-slate-900/40 text-slate-400 border-slate-800/60"
                }`}
              >
                {t("languages.en")}
              </button>
              <button
                onClick={() => i18n.changeLanguage("es")}
                className={`px-2 py-1 rounded text-xs border ${
                  currentLanguage.startsWith("es")
                    ? "bg-cyan-500/10 text-cyan-400 border-cyan-500/30 font-bold"
                    : "bg-slate-900/40 text-slate-400 border-slate-800/60"
                }`}
              >
                {t("languages.es")}
              </button>
            </div>
          </div>
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => handleTabChange(tab.id)}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl border text-sm font-semibold transition-all ${
                  isActive
                    ? "bg-cyan-500/10 text-cyan-400 border-cyan-500/30"
                    : "bg-slate-900/40 text-slate-400 border-slate-800/60 hover:text-white"
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
          <div className="text-[10px] uppercase font-bold text-slate-500 tracking-widest pl-3 mb-3">
            {t("nav.navigation")}
          </div>
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => handleTabChange(tab.id)}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl border text-sm font-semibold transition-all text-left ${
                  isActive
                    ? "bg-cyan-500/10 text-cyan-400 border-cyan-500/30 border-glow-cyan"
                    : "bg-slate-900/30 text-slate-400 border-transparent hover:text-slate-200 hover:bg-slate-900/50"
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
          {activeTab === "overview" && (
            <div className="space-y-8 animate-fadeIn">
              {/* Hero Banner card */}
              <div className="relative rounded-2xl overflow-hidden border border-slate-800/80 bg-gradient-to-br from-slate-900 via-slate-950 to-slate-900 p-8 md:p-10 shadow-2xl">
                <div className="absolute top-0 right-0 w-64 h-64 bg-cyan-500/5 rounded-full filter blur-3xl pointer-events-none"></div>
                <div className="max-w-2xl space-y-4">
                  <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    <CheckCircle className="w-3.5 h-3.5" /> {t("overview.hero.badge")}
                  </span>
                  <h2 className="text-3xl md:text-4xl font-extrabold text-white tracking-tight leading-tight">
                    {t("overview.hero.title")}{" "}
                    <span className="text-gradient-cyan">{t("overview.hero.title_highlight")}</span>
                  </h2>
                  <p className="text-slate-400 text-sm md:text-base leading-relaxed">
                    {t("overview.hero.desc")}
                  </p>
                  <div className="flex flex-wrap gap-4 pt-2">
                    <button
                      onClick={() => handleTabChange("architecture")}
                      className="px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-cyan-600 hover:from-cyan-400 hover:to-cyan-500 text-slate-950 font-bold text-sm rounded-lg shadow-lg shadow-cyan-500/10 transition-all flex items-center gap-1.5 focus:outline-none"
                    >
                      {t("overview.hero.btn_explore")}
                      <ArrowRight className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => handleTabChange("config")}
                      className="px-5 py-2.5 bg-slate-900 border border-slate-800 hover:border-slate-700 font-semibold text-slate-200 text-sm rounded-lg hover:bg-slate-850 transition-all flex items-center gap-1.5 focus:outline-none"
                    >
                      {t("overview.hero.btn_config")}
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
                  <h3 className="text-base font-bold text-white">
                    {t("overview.features.tokio.title")}
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    {t("overview.features.tokio.desc")}
                  </p>
                </div>

                <div className="p-6 rounded-xl glass-panel glass-panel-hover space-y-2">
                  <div className="w-10 h-10 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center justify-center mb-4">
                    <Settings className="w-5 h-5" />
                  </div>
                  <h3 className="text-base font-bold text-white">
                    {t("overview.features.config.title")}
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    {t("overview.features.config.desc")}
                  </p>
                </div>

                <div className="p-6 rounded-xl glass-panel glass-panel-hover space-y-2">
                  <div className="w-10 h-10 rounded-lg bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 flex items-center justify-center mb-4">
                    <Key className="w-5 h-5" />
                  </div>
                  <h3 className="text-base font-bold text-white">
                    {t("overview.features.transfer.title")}
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    {t("overview.features.transfer.desc")}
                  </p>
                </div>
              </div>

              {/* Getting Started Guide */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Terminal className="text-emerald-400 w-5 h-5" />
                  {t("overview.quickstart.title")}
                </h3>
                <p className="text-slate-400 text-sm">{t("overview.quickstart.desc")}</p>
                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-emerald-400/90 space-y-2 overflow-x-auto">
                  <div>
                    <span className="text-slate-500">{t("overview.quickstart.clone")}</span>
                  </div>
                  <div>{CLONE_CMD}</div>
                  <div>{CD_CMD}</div>
                  <div>
                    <span className="text-slate-500">{t("overview.quickstart.build")}</span>
                  </div>
                  <div>{BUILD_CMD}</div>
                  <div>
                    <span className="text-slate-500">{t("overview.quickstart.run")}</span>
                  </div>
                  <div>{RUN_CMD}</div>
                </div>
              </div>
            </div>
          )}

          {/* Tab 2: Architecture */}
          {activeTab === "architecture" && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <Cpu className="text-cyan-400 w-6 h-6" />
                  {t("architecture.title")}
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">{t("architecture.desc")}</p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Loop splitting */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
                    {t("architecture.splitting.title")}
                  </h3>
                  <p
                    className="text-slate-400 text-xs leading-relaxed text-slate-400"
                    dangerouslySetInnerHTML={{ __html: t("architecture.splitting.desc") }}
                  />
                </div>

                {/* World State */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    {t("architecture.state.title")}
                  </h3>
                  <p
                    className="text-slate-400 text-xs leading-relaxed text-slate-400"
                    dangerouslySetInnerHTML={{ __html: t("architecture.state.desc") }}
                  />
                </div>
              </div>

              {/* Chat broadcast and Operators details */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white">
                  {t("architecture.broadcasting.title")}
                </h3>
                <p
                  className="text-slate-400 text-sm leading-relaxed"
                  dangerouslySetInnerHTML={{ __html: t("architecture.broadcasting.desc") }}
                />
                <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800 text-xs space-y-2">
                  <div className="font-mono text-cyan-400">
                    {t("architecture.broadcasting.flow_title")}
                  </div>
                  <ol className="list-decimal list-inside text-slate-400 space-y-1 font-mono text-[11px]">
                    <li>{t("architecture.broadcasting.step_1")}</li>
                    <li>{t("architecture.broadcasting.step_2")}</li>
                    <li>{t("architecture.broadcasting.step_3")}</li>
                    <li>{t("architecture.broadcasting.step_4")}</li>
                    <li>{t("architecture.broadcasting.step_5")}</li>
                    <li>{t("architecture.broadcasting.step_6")}</li>
                  </ol>
                </div>
              </div>
            </div>
          )}

          {/* Tab: World & Persistence */}
          {activeTab === "world" && (
            <div className="space-y-6 animate-fadeIn">
              {/* Header Card */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <Globe className="text-cyan-400 w-6 h-6" />
                  {t("world.title")}
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">{t("world.desc")}</p>
              </div>

              {/* World Generation Section */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div className="p-6 rounded-xl glass-panel space-y-4">
                  <h3 className="text-lg font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
                    {t("world.generators.title")}
                  </h3>
                  <div className="space-y-3 text-slate-400 text-xs leading-relaxed">
                    <p dangerouslySetInnerHTML={{ __html: t("world.generators.desc") }} />
                    <ul className="list-disc list-inside space-y-2 pl-2">
                      <li dangerouslySetInnerHTML={{ __html: t("world.generators.flat") }} />
                      <li dangerouslySetInnerHTML={{ __html: t("world.generators.normal") }} />
                    </ul>
                  </div>
                </div>

                <div className="p-6 rounded-xl glass-panel space-y-4">
                  <h3 className="text-lg font-bold text-white flex items-center gap-2">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    {t("world.spawn.title")}
                  </h3>
                  <div className="space-y-3 text-slate-400 text-xs leading-relaxed">
                    <p dangerouslySetInnerHTML={{ __html: t("world.spawn.desc") }} />
                    <ol className="list-decimal list-inside space-y-2 pl-2 font-mono text-[11px]">
                      <li>{t("world.spawn.step_1")}</li>
                      <li>{t("world.spawn.step_2")}</li>
                      <li>{t("world.spawn.step_3")}</li>
                      <li>{t("world.spawn.step_4")}</li>
                      <li>{t("world.spawn.step_5")}</li>
                    </ol>
                  </div>
                </div>
              </div>

              {/* Persistence Details */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Database className="text-cyan-400 w-5 h-5" />
                  {t("world.persistence.title")}
                </h3>
                <p
                  className="text-slate-400 text-sm leading-relaxed"
                  dangerouslySetInnerHTML={{ __html: t("world.persistence.desc") }}
                />

                <div className="grid grid-cols-1 md:grid-cols-3 gap-4 pt-2">
                  <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800/60 text-xs space-y-2">
                    <div className="font-bold text-white font-mono">
                      {t("world.persistence.meta_title")}
                    </div>
                    <p className="text-slate-400 leading-normal">
                      {t("world.persistence.meta_desc")}
                    </p>
                  </div>
                  <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800/60 text-xs space-y-2">
                    <div className="font-bold text-white font-mono">
                      {t("world.persistence.chunk_title")}
                    </div>
                    <p className="text-slate-400 leading-normal">
                      {t("world.persistence.chunk_desc")}
                    </p>
                  </div>
                  <div className="p-4 rounded-lg bg-slate-950/40 border border-slate-800/60 text-xs space-y-2">
                    <div className="font-bold text-white font-mono">
                      {t("world.persistence.io_title")}
                    </div>
                    <p className="text-slate-400 leading-normal">
                      {t("world.persistence.io_desc")}
                    </p>
                  </div>
                </div>

                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-emerald-400/90 space-y-1 overflow-x-auto">
                  <div className="text-slate-500">{t("world.persistence.layout_title")}</div>
                  <div>{WORLD_DIR_NAME}</div>
                  <div>
                    {LEVEL_JSON}
                    <span className="text-slate-500">{t("world.persistence.comment_meta")}</span>
                  </div>
                  <div>
                    {CHUNKS_DIR}
                    <span className="text-slate-500">{t("world.persistence.comment_chunks")}</span>
                  </div>
                  <div>
                    {CHUNK_0_0}
                    <span className="text-slate-500">{t("world.persistence.comment_c00")}</span>
                  </div>
                  <div>
                    {CHUNK_0_1}
                    <span className="text-slate-500">{t("world.persistence.comment_c01")}</span>
                  </div>
                  <div>
                    {CHUNK_M1_0}
                    <span className="text-slate-500">{t("world.persistence.comment_c10")}</span>
                  </div>
                </div>
              </div>

              {/* Background Loops & Backups */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Terminal className="text-emerald-400 w-5 h-5" />
                  {t("world.tasks.title")}
                </h3>
                <p className="text-slate-400 text-sm leading-relaxed">{t("world.tasks.desc")}</p>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div className="p-5 rounded-lg bg-slate-950/40 border border-slate-800/60 space-y-3">
                    <h4 className="text-sm font-bold text-white">
                      {t("world.tasks.autosave_title")}
                    </h4>
                    <p
                      className="text-slate-400 text-xs leading-relaxed"
                      dangerouslySetInnerHTML={{ __html: t("world.tasks.autosave_desc") }}
                    />
                  </div>

                  <div className="p-5 rounded-lg bg-slate-950/40 border border-slate-800/60 space-y-3">
                    <h4 className="text-sm font-bold text-white">
                      {t("world.tasks.backup_title")}
                    </h4>
                    <p
                      className="text-slate-400 text-xs leading-relaxed"
                      dangerouslySetInnerHTML={{ __html: t("world.tasks.backup_desc") }}
                    />
                  </div>
                </div>

                <div className="p-5 rounded-lg bg-slate-950/40 border border-slate-800/60 space-y-3">
                  <h4 className="text-sm font-bold text-white">{t("world.tasks.prune_title")}</h4>
                  <p
                    className="text-slate-400 text-xs leading-relaxed"
                    dangerouslySetInnerHTML={{ __html: t("world.tasks.prune_desc") }}
                  />
                </div>
              </div>
            </div>
          )}

          {/* Tab 3: Config Generator */}
          {activeTab === "config" && <ConfigGenerator />}

          {/* Tab 4: Protocol & Packets */}
          {activeTab === "protocol" && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-3">
                <h2 className="text-2xl font-bold text-white">{t("protocol_tab.title")}</h2>
                <p className="text-slate-400 text-sm leading-relaxed">{t("protocol_tab.desc")}</p>

                {/* Visual diagram representation */}
                <div className="flex flex-col md:flex-row justify-between items-center gap-4 py-6 px-4 rounded-lg bg-slate-950/40 border border-slate-800 text-xs font-mono text-center">
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-cyan-500/20 bg-cyan-950/10 text-cyan-300">
                    <span className="block font-bold">
                      {t("protocol_tab.flow.handshake_title")}
                    </span>
                    <span className="text-[10px] text-slate-500">
                      {t("protocol_tab.flow.handshake_desc")}
                    </span>
                  </div>
                  <div className="text-slate-500">➔</div>
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-emerald-500/20 bg-emerald-950/10 text-emerald-300">
                    <span className="block font-bold">{t("protocol_tab.flow.status_title")}</span>
                    <span className="text-[10px] text-slate-500">
                      {t("protocol_tab.flow.status_desc")}
                    </span>
                  </div>
                  <div className="text-slate-500">{t("protocol_tab.flow.or_arrow")}</div>
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-cyan-500/20 bg-cyan-950/10 text-cyan-300">
                    <span className="block font-bold">{t("protocol_tab.flow.login_title")}</span>
                    <span className="text-[10px] text-slate-500">
                      {t("protocol_tab.flow.login_desc")}
                    </span>
                  </div>
                  <div className="text-slate-500">➔</div>
                  <div className="w-full md:w-1/5 p-3 rounded-lg border border-cyan-500/20 bg-cyan-950/10 text-cyan-300">
                    <span className="block font-bold">{t("protocol_tab.flow.play_title")}</span>
                    <span className="text-[10px] text-slate-500">
                      {t("protocol_tab.flow.play_desc")}
                    </span>
                  </div>
                </div>
              </div>

              <PacketReference />
            </div>
          )}

          {/* Tab 5: Transfer Protocol */}
          {activeTab === "transfer" && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <Key className="text-cyan-400 w-6 h-6" />
                  {t("transfer_tab.title")}
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">{t("transfer_tab.desc")}</p>
              </div>

              {/* Steps grid */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
                <div className="p-5 rounded-xl glass-panel space-y-2">
                  <span className="text-lg font-mono text-cyan-400 font-bold">01</span>
                  <h3 className="text-sm font-bold text-white">
                    {t("transfer_tab.steps.negotiate_title")}
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    {t("transfer_tab.steps.negotiate_desc")}
                  </p>
                </div>

                <div className="p-5 rounded-xl glass-panel space-y-2">
                  <span className="text-lg font-mono text-emerald-400 font-bold">02</span>
                  <h3 className="text-sm font-bold text-white">
                    {t("transfer_tab.steps.store_title")}
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    {t("transfer_tab.steps.store_desc")}
                  </p>
                </div>

                <div className="p-5 rounded-xl glass-panel space-y-2">
                  <span className="text-lg font-mono text-cyan-400 font-bold">03</span>
                  <h3 className="text-sm font-bold text-white">
                    {t("transfer_tab.steps.handoff_title")}
                  </h3>
                  <p className="text-slate-400 text-xs leading-relaxed">
                    {t("transfer_tab.steps.handoff_desc")}
                  </p>
                </div>
              </div>

              {/* Code/Flow explanation card */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <FileText className="text-cyan-400 w-4 h-4" />
                  {t("transfer_tab.flow_card.title")}
                </h3>
                <p className="text-slate-400 text-sm">{t("transfer_tab.flow_card.desc")}</p>
                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-slate-300 leading-relaxed overflow-x-auto">
                  <div className="text-slate-500">{t("transfer_tab.flow_card.comment")}</div>
                  <div>{CODE_LINE_1}</div>
                  <div>{CODE_LINE_2}</div>
                  <div>{CODE_LINE_3}</div>
                  <div>{CODE_LINE_4}</div>
                  <div>{CODE_LINE_5}</div>
                  <div>{CODE_LINE_6}</div>
                  <div>{CODE_LINE_7}</div>
                  <div>{CODE_LINE_8}</div>
                </div>
              </div>
            </div>
          )}

          {/* Tab 6: Security & Ops */}
          {activeTab === "security" && (
            <div className="space-y-6 animate-fadeIn">
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h2 className="text-2xl font-bold text-white flex items-center gap-2">
                  <ShieldAlert className="text-emerald-400 w-6 h-6" />
                  {t("security_tab.title")}
                </h2>
                <p className="text-slate-400 text-sm leading-relaxed">{t("security_tab.desc")}</p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Rate Limiting */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white">
                    {t("security_tab.rate_limiting.title")}
                  </h3>
                  <p
                    className="text-slate-400 text-xs leading-relaxed text-slate-400"
                    dangerouslySetInnerHTML={{ __html: t("security_tab.rate_limiting.desc") }}
                  />
                </div>

                {/* Operator roles */}
                <div className="p-5 rounded-xl glass-panel space-y-3">
                  <h3 className="text-base font-bold text-white">{t("security_tab.ops.title")}</h3>
                  <p
                    className="text-slate-400 text-xs leading-relaxed text-slate-400"
                    dangerouslySetInnerHTML={{ __html: t("security_tab.ops.desc") }}
                  />
                </div>
              </div>

              {/* Ops configuration file syntax */}
              <div className="p-6 rounded-xl glass-panel space-y-4">
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <Terminal className="text-cyan-400 w-4 h-4" />
                  {t("security_tab.file_format.title")}
                </h3>
                <div className="bg-slate-950/60 rounded-lg border border-slate-800 p-4 font-mono text-xs text-emerald-400/90 leading-relaxed overflow-x-auto">
                  <div>{OPS_HEADER}</div>
                  <div>{OPS_UUID}</div>
                  <div>{OPS_NAME}</div>
                  <div>{OPS_LEVEL}</div>
                  <div>{OPS_BYPASS}</div>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>

      {/* Page Footer */}
      <footer className="mt-auto bg-slate-950/60 border-t border-slate-800/80 px-6 py-6 text-center text-xs text-slate-500 z-10">
        <p>{t("footer.copyright")}</p>
      </footer>
    </div>
  );
}
