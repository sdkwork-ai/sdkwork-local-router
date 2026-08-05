import React, { useState, useEffect } from 'react';
import { X, Box, TerminalSquare, CheckCircle2 } from 'lucide-react';
import { AgentTool } from '@sdkwork/modelkit-types';

interface InstallSkillModalProps {
  isOpen: boolean;
  skillName: string;
  agents: AgentTool[];
  initialSelectedAgents?: string[];
  onClose: () => void;
  onConfirm: (selectedAgents: string[]) => void;
}

export function InstallSkillModal({ isOpen, skillName, agents, initialSelectedAgents = [], onClose, onConfirm }: InstallSkillModalProps) {
  const [selectedAgents, setSelectedAgents] = useState<string[]>([]);

  useEffect(() => {
    if (isOpen) {
      setSelectedAgents(initialSelectedAgents);
    }
  }, [isOpen, initialSelectedAgents]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-center items-center bg-black/75 backdrop-blur-sm p-4">
      <div className="bg-surface border border-divider rounded-2xl w-full max-w-2xl shadow-2xl flex flex-col overflow-hidden animate-in fade-in zoom-in duration-200">
        <div className="px-6 py-5 border-b border-divider flex justify-between items-center bg-panel">
          <h2 className="text-md font-bold text-text-main flex items-center gap-2">
            <Box size={18} className="text-red-500" />
            Install Plugin to Agents
          </h2>
          <button onClick={onClose} className="text-text-muted hover:text-text-main transition-colors">
            <X size={20} />
          </button>
        </div>
        <div className="p-6">
          <p className="text-[14px] text-text-muted mb-5 font-medium leading-relaxed">
            Select agents to equip with the <span className="text-red-400 font-bold">{skillName}</span> plugin:
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-h-[60vh] overflow-y-auto pr-2 custom-scrollbar">
            {agents.map(agent => (
              <label key={agent.id} className="flex items-center justify-between p-4 rounded-xl border border-divider bg-surface hover:bg-surface-hover hover:border-[var(--color-primary-alpha)] cursor-pointer transition-all group shadow-sm">
                <div className="flex items-center gap-3.5">
                  <div className="w-9 h-9 rounded-lg bg-surface-hover border border-divider-strong flex items-center justify-center text-lg shadow-inner"><TerminalSquare size={16} className="text-primary-main" /></div>
                  <span className="text-sm font-semibold text-text-main group-hover:text-text-main transition-colors">{agent.name}</span>
                </div>
                <div className="relative flex items-center justify-center">
                  <input 
                    type="checkbox" 
                    className="peer appearance-none w-5 h-5 border-2 border-divider-strong rounded-md checked:bg-white checked:border-white transition-colors cursor-pointer"
                    checked={selectedAgents.includes(agent.id)}
                    onChange={(e) => {
                      if (e.target.checked) {
                        setSelectedAgents([...selectedAgents, agent.id]);
                      } else {
                        setSelectedAgents(selectedAgents.filter(id => id !== agent.id));
                      }
                    }}
                  />
                  <CheckCircle2 size={14} className="absolute text-text-main pointer-events-none opacity-0 peer-checked:opacity-100 transition-opacity" />
                </div>
              </label>
            ))}
          </div>
        </div>
        <div className="px-6 py-5 border-t border-divider bg-panel flex justify-end gap-3">
          <button 
            onClick={onClose} 
            className="px-5 py-2.5 text-[13px] font-bold text-text-muted hover:text-text-main transition-colors rounded-lg hover:bg-black/5 dark:hover:bg-white/5"
          >
            Cancel
          </button>
          <button 
            onClick={() => onConfirm(selectedAgents)} 
            className={`px-6 py-2.5 rounded-lg text-[13px] font-bold transition-all shadow-lg ${
              selectedAgents.length > 0 
                ? 'bg-surface text-text-main hover:scale-[1.03]' 
                : 'bg-surface-hover text-text-muted cursor-not-allowed border border-divider-strong'
            }`}
            disabled={selectedAgents.length === 0}
          >
            Confirm Setup
          </button>
        </div>
      </div>
    </div>
  );
}
