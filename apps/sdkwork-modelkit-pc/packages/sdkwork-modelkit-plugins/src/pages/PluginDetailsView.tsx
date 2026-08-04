import React, { useState } from 'react';
import { Download, Star, Settings, Key, Shield, Box, ArrowLeft, GitBranch, FolderArchive, CheckCircle2, Plug, Copy, Check } from 'lucide-react';
import { useAppContext } from '@sdkwork/modelkit-core';
import { PluginItem } from '../services/types';
import { AgentTool } from '@sdkwork/modelkit-types';
import { toast } from 'sonner';

interface PluginDetailsViewProps {
  plugin: PluginItem;
  agents: AgentTool[];
  onBack: () => void;
  onInstallClick: (plugin: PluginItem) => void;
  renderIcon: (iconName: string, className?: string) => React.ReactNode;
}

export function PluginDetailsView({ plugin, agents, onBack, onInstallClick, renderIcon }: PluginDetailsViewProps) {
  const { t } = useAppContext();
  const [installTab, setInstallTab] = useState<'agent'|'npm'|'pnpm'|'bun'>('agent');
  const [agentTool, setAgentTool] = useState<'openclaw'|'codex'>('openclaw');
  const [mirror, setMirror] = useState<'official'|'taobao'>('official');
  const [isCopied, setIsCopied] = useState(false);

  const getInstallCommand = () => {
    const version = "1.0.0";
    const registry = mirror === 'official' ? 'https://registry.cloudhub.io' : 'https://registry.npmmirror.com';
    const npmRegistry = mirror === 'taobao' ? ' --registry=https://registry.npmmirror.com' : '';
    
    if (installTab === 'agent') {
      if (agentTool === 'openclaw') {
        return `Find via cloudhub cloudhub://plugins/${plugin.name}@${version} plugin, using registry ${registry}。`;
      } else {
        return `Find via codex codex://plugins/${plugin.name}@${version} plugin, using registry ${registry.replace('cloudhub.io', 'codex.io')}。`;
      }
    }
    if (installTab === 'npm') return `npm install ${plugin.name}@${version}${npmRegistry}`;
    if (installTab === 'pnpm') return `pnpm add ${plugin.name}@${version}${npmRegistry}`;
    if (installTab === 'bun') return `bun add ${plugin.name}@${version}${npmRegistry}`;
    return '';
  };

  const handleCopyCommand = () => {
    navigator.clipboard.writeText(getInstallCommand());
    setIsCopied(true);
    toast.success(t('common:copied', 'Copied to clipboard!'));
    setTimeout(() => setIsCopied(false), 2000);
  };

  return (
    <div className="w-full mx-auto p-6 lg:p-8">
      <button 
        onClick={onBack} 
        className="flex items-center gap-2 text-text-muted hover:text-text-main transition-colors mb-10 group"
      >
        <ArrowLeft size={18} className="group-hover:-translate-x-1 transition-transform" />
        <span className="font-medium">{t('plugins:back_to_plugins')}</span>
      </button>

      <div className="flex items-start justify-between gap-8 mb-12">
        <div className="flex items-center gap-6">
          <div className="w-24 h-24 rounded-2xl bg-panel border border-divider flex items-center justify-center shrink-0 shadow-lg">
            {renderIcon(plugin.icon, "text-emerald-500 w-10 h-10")}
          </div>
          <div>
            <h1 className="text-4xl font-bold text-text-main mb-3 tracking-tight">{plugin.name}</h1>
            <p className="text-lg text-text-muted font-medium">{t('plugins:by')} <span className="text-text-main">{plugin.author}</span></p>
          </div>
        </div>
        <button 
          onClick={() => onInstallClick(plugin)}
          className="px-8 py-3.5 rounded-xl bg-surface text-text-main font-bold text-lg transition-all shadow-[0_0_20px_rgba(255,255,255,0.15)] hover:scale-105 transform flex items-center gap-2"
        >
          <Plug size={20} />
          {plugin.installedAgents.length > 0 ? t('plugins:manage_integration') : t('plugins:install_plugin')}
        </button>
      </div>

      <div className="flex items-center gap-12 mb-14 text-sm text-text-muted border-y border-divider py-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-surface-hover flex items-center justify-center text-yellow-500">
            <Star size={20} />
          </div>
          <div>
            <div className="text-text-main text-lg font-bold">{plugin.rating}k</div>
            <div>Stars</div>
          </div>
        </div>
        <div className="w-px h-10 bg-surface-hover"></div>
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-surface-hover flex items-center justify-center text-primary-light">
            <Download size={20} />
          </div>
          <div>
            <div className="text-text-main text-lg font-bold">{plugin.downloads}</div>
            <div>Downloads</div>
          </div>
        </div>
        <div className="w-px h-10 bg-surface-hover"></div>
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-surface-hover flex items-center justify-center text-primary-light">
            <Box size={20} />
          </div>
          <div>
            <div className="text-text-main text-lg font-bold uppercase">{plugin.type}</div>
            <div>Category</div>
          </div>
        </div>
      </div>

      {/* Installation Banner */}
      <div className="mb-12">
        <h3 className="text-sm font-medium text-text-main mb-4">{t("plugins:search_desc", "Search plugins. Supports version control, rollback anytime.")}</h3>
        <div className="bg-surface border border-divider rounded-2xl p-5 shadow-sm">
          <div className="flex items-center justify-between mb-5 flex-wrap gap-4">
            <div className="flex items-center gap-4 text-xs font-semibold text-text-main">
              <span>{t("plugins:one_click_install", "One-click install any plugin package:")}</span>
              
              {installTab === 'agent' && (
                <div className="flex items-center gap-2 bg-canvas px-3 py-1.5 rounded-lg border border-divider">
                  <span className="text-text-muted">{t("plugins:tool_label", "Tool:")}</span>
                  <select 
                    value={agentTool}
                    onChange={e => setAgentTool(e.target.value as any)}
                    className="bg-transparent outline-none cursor-pointer"
                  >
                    <option value="openclaw">openclaw</option>
                    <option value="codex">codex</option>
                  </select>
                </div>
              )}

              <div className="flex items-center gap-2 bg-canvas px-3 py-1.5 rounded-lg border border-divider">
                <span className="text-text-muted">{t("plugins:mirror_label", "Mirror:")}</span>
                <select 
                  value={mirror}
                  onChange={e => setMirror(e.target.value as any)}
                  className="bg-transparent outline-none cursor-pointer"
                >
                  <option value="official">{t("plugins:official_default", "Official Default")}</option>
                  <option value="taobao">{t("plugins:taobao_mirror", "Taobao Mirror")}</option>
                </select>
              </div>
            </div>
            
            <div className="flex bg-canvas border border-divider p-1 rounded-full text-xs font-bold">
              <button onClick={() => setInstallTab('agent')} className={`px-5 py-1.5 rounded-full transition-all ${installTab === 'agent' ? 'bg-[#ff6b4a] text-white shadow-sm' : 'text-text-muted hover:text-text-main hover:bg-surface-hover'}`}>{t("plugins:agent_tab", "Agent")}</button>
              <button onClick={() => setInstallTab('npm')} className={`px-5 py-1.5 rounded-full transition-all ${installTab === 'npm' ? 'bg-surface shadow-[0_2px_8px_rgba(0,0,0,0.08)] text-text-main' : 'text-text-muted hover:text-text-main hover:bg-surface-hover'}`}>npm</button>
              <button onClick={() => setInstallTab('pnpm')} className={`px-5 py-1.5 rounded-full transition-all ${installTab === 'pnpm' ? 'bg-surface shadow-[0_2px_8px_rgba(0,0,0,0.08)] text-text-main' : 'text-text-muted hover:text-text-main hover:bg-surface-hover'}`}>pnpm</button>
              <button onClick={() => setInstallTab('bun')} className={`px-5 py-1.5 rounded-full transition-all ${installTab === 'bun' ? 'bg-surface shadow-[0_2px_8px_rgba(0,0,0,0.08)] text-text-main' : 'text-text-muted hover:text-text-main hover:bg-surface-hover'}`}>bun</button>
            </div>
          </div>

          <div className="group relative bg-canvas border border-divider-strong rounded-xl p-4 font-mono text-[13px] text-text-main/80 flex items-center justify-between">
            <span className="break-all pr-8">
              {installTab === 'agent' ? (
                <>
                  {t("plugins:find_via", "Find via")} {agentTool} {agentTool}://plugins/{plugin.name}@1.0.0 plugin, using registry <span className="bg-surface px-1.5 py-0.5 rounded border border-divider text-text-main">{mirror === 'official' ? (agentTool === 'openclaw' ? 'https://registry.cloudhub.io' : 'https://registry.codex.io') : 'https://registry.npmmirror.com'}</span>.
                </>
              ) : (
                getInstallCommand()
              )}
            </span>
            <button 
              onClick={handleCopyCommand}
              className="absolute right-3 top-1/2 -translate-y-1/2 p-2 rounded-lg bg-surface border border-divider text-text-muted hover:text-primary-main hover:border-primary-main/50 transition-all opacity-0 group-hover:opacity-100"
              title="Copy to clipboard"
            >
              {isCopied ? <Check size={14} className="text-emerald-500" /> : <Copy size={14} />}
            </button>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-12">
        <div className="col-span-2 space-y-12">
          <section>
            <h2 className="text-xl font-bold text-text-main mb-4">Overview</h2>
            <p className="text-[16px] text-text-main leading-relaxed font-medium">{plugin.desc}</p>
          </section>
          
          <section>
            <h2 className="text-xl font-bold text-text-main mb-4">Endpoints</h2>
            <div className="space-y-3">
              {plugin.endpoints.map((ep, i) => (
                <div key={i} className="flex items-center gap-4 bg-surface border border-divider rounded-xl p-4 shadow-sm">
                  <span className={`px-2.5 py-1 rounded text-xs font-bold font-mono ${
                    ep.method === 'GET' ? 'bg-primary-main/10 text-primary-light border border-primary-main/10' :
                    ep.method === 'POST' ? 'bg-green-500/10 text-green-400 border border-green-500/10' :
                    'bg-gray-500/10 text-text-muted border border-gray-500/10'
                  }`}>
                    {ep.method}
                  </span>
                  <span className="font-mono text-sm text-text-main">{ep.path}</span>
                  <span className="text-sm text-text-muted ml-auto">{ep.desc}</span>
                </div>
              ))}
            </div>
          </section>
        </div>

        <div className="space-y-8">
          <div className="bg-surface border border-divider rounded-2xl p-6 shadow-sm">
            <h3 className="text-sm font-bold text-text-main uppercase tracking-widest mb-5 flex items-center gap-2">
              <Settings size={16} className="text-text-muted" /> Specs
            </h3>
            <div className="space-y-5">
              <div className="flex flex-col gap-2">
                <span className="text-text-muted text-xs font-semibold uppercase tracking-wider">Schema Standard</span>
                <span className="text-text-main font-mono text-sm bg-surface px-3 py-1.5 rounded-lg inline-block w-fit border border-divider">{plugin.schemaType}</span>
              </div>
              <div className="flex flex-col gap-2 pt-2 border-t border-divider">
                <span className="text-text-muted text-xs font-semibold uppercase tracking-wider">Authentication</span>
                <span className="text-text-main font-medium text-sm flex items-center gap-2 bg-surface px-3 py-1.5 rounded-lg w-fit border border-divider">
                  <Key size={14} className="text-amber-400"/> {plugin.authType}
                </span>
              </div>
              {/* @ts-ignore */}
              {plugin.sourceType && (
                <div className="flex flex-col gap-2 pt-2 border-t border-divider">
                  <span className="text-text-muted text-xs font-semibold uppercase tracking-wider">{t('plugins:txt_1145')}</span>
                  {/* @ts-ignore */}
                  {plugin.sourceType === 'repo' ? (
                    <a 
                      /* @ts-ignore */
                      href={plugin.repoUrl} 
                      target="_blank" 
                      rel="noreferrer"
                      className="text-emerald-400 font-medium text-xs hover:underline flex items-center gap-1.5 bg-surface px-3 py-1.5 rounded-lg w-fit border border-divider transition-colors"
                    >
                      <GitBranch size={13}/> Git Repository
                    </a>
                  ) : (
                    <span className="text-text-main font-medium text-xs flex items-center gap-1.5 bg-surface px-3 py-1.5 rounded-lg w-fit border border-divider">
                      {/* @ts-ignore */}
                      <FolderArchive size={13} className="text-emerald-400"/> {plugin.zipFileName}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>

          <div className="bg-surface border border-divider rounded-2xl p-6 shadow-sm">
            <h3 className="text-sm font-bold text-text-main uppercase tracking-widest mb-5 flex items-center gap-2">
              <Shield size={16} className="text-emerald-500" /> Permissions
            </h3>
            <ul className="space-y-3">
              {plugin.permissions.map((perm, i) => (
                <li key={i} className="flex items-center gap-3 text-sm text-text-main font-medium bg-surface border border-divider px-3 py-2 rounded-lg">
                  <CheckCircle2 size={16} className="text-emerald-500" /> {perm}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
