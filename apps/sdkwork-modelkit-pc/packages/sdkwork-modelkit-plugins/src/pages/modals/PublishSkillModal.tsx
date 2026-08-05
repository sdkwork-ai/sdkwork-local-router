import React, { useState, useRef } from 'react';
import { X, Blocks, TerminalSquare, GitBranch, Upload, FolderArchive } from 'lucide-react';
import { toast } from 'sonner';
import { skillhubService } from '../../services/SkillhubService';
import { SkillItem } from '../../services/types';

interface PublishSkillModalProps {
  isOpen: boolean;
  onClose: () => void;
  categories: string[];
  onSkillPublished: (newSkill: SkillItem) => void;
}

export function PublishSkillModal({ isOpen, onClose, categories, onSkillPublished }: PublishSkillModalProps) {
  const [newPluginName, setNewPluginName] = useState('');
  const [newPluginAuthor, setNewPluginAuthor] = useState('');
  const [newPluginCategory, setNewPluginCategory] = useState('Workflows');
  const [newPluginDesc, setNewPluginDesc] = useState('');
  const [newPluginPermissions, setNewPluginPermissions] = useState('');
  const [newPluginSchemaType, setNewPluginSchemaType] = useState('REST API');
  const [newPluginAuthType, setNewPluginAuthType] = useState('None');

  const [publishSourceType, setPublishSourceType] = useState<'repo' | 'zip'>('repo');
  const [pluginRepoUrl, setPluginRepoUrl] = useState('');
  const [pluginZipFile, setPluginZipFile] = useState<File | null>(null);
  const [pluginZipFileName, setPluginZipFileName] = useState('');
  
  const [isDragging, setIsDragging] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  if (!isOpen) return null;

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      if (file.name.toLowerCase().endsWith('.zip')) {
        setPluginZipFile(file);
        setPluginZipFileName(file.name);
        toast.success(`Selected file: ${file.name}`);
      } else {
        toast.error('Only .zip files are supported');
      }
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      const file = e.target.files[0];
      if (file.name.toLowerCase().endsWith('.zip')) {
        setPluginZipFile(file);
        setPluginZipFileName(file.name);
        toast.success(`Selected file: ${file.name}`);
      } else {
        toast.error('Only .zip files are supported');
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPluginName || !newPluginAuthor || !newPluginDesc) {
      toast.error('Please fill in Name, Author, and Description');
      return;
    }
    if (publishSourceType === 'repo' && !pluginRepoUrl) {
      toast.error('Please fill in the Repository URL');
      return;
    }
    if (publishSourceType === 'zip' && !pluginZipFileName) {
      toast.error('Please upload a ZIP compressed package');
      return;
    }

    try {
      const newSkill = await skillhubService.publishSkill({
        name: newPluginName,
        author: newPluginAuthor.startsWith('@') ? newPluginAuthor : `@${newPluginAuthor}`,
        category: newPluginCategory,
        desc: newPluginDesc,
        schemaType: newPluginSchemaType || 'REST API',
        authType: newPluginAuthType || 'None',
        permissions: newPluginPermissions ? newPluginPermissions.split(',').map(p => p.trim()) : ['Memory Access']
      });

      onSkillPublished(newSkill);
      onClose();
      toast.success('Plugin published successfully');
    } catch(err) {
      toast.error('Publishing failed');
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center items-center bg-black/80 backdrop-blur-sm p-4">
      <div className="bg-panel border border-divider rounded-2xl w-full max-w-lg shadow-2xl flex flex-col max-h-[85vh] animate-in fade-in zoom-in duration-200">
        <div className="px-5 py-4 border-b border-divider flex justify-between items-center bg-canvas rounded-t-2xl shrink-0">
          <div className="flex items-center gap-2">
            <Blocks size={18} className="text-emerald-500" />
            <h2 className="text-sm font-bold text-text-main">Publish Dynamic Tool/Plugin</h2>
          </div>
          <button onClick={onClose} className="p-1 rounded-md text-text-muted hover:text-text-main hover:bg-surface-hover transition-colors">
            <X size={16} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 custom-scrollbar">
          <form id="publishForm" onSubmit={handleSubmit} className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <label className="text-xs font-bold text-text-main">Plugin/Tool Name <span className="text-red-500">*</span></label>
                <input 
                  type="text" 
                  value={newPluginName}
                  onChange={(e) => setNewPluginName(e.target.value)}
                  placeholder="e.g. GitHub Agent"
                  className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-all placeholder:text-zinc-500"
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-bold text-text-main">Author ID <span className="text-red-500">*</span></label>
                <input 
                  type="text" 
                  value={newPluginAuthor}
                  onChange={(e) => setNewPluginAuthor(e.target.value)}
                  placeholder="e.g. @developer"
                  className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-all placeholder:text-zinc-500"
                />
              </div>
            </div>

            <div className="space-y-1.5">
               <label className="text-xs font-bold text-text-main">Function Source Type <span className="text-red-500">*</span></label>
               <div className="flex gap-2">
                 <button
                   type="button"
                   onClick={() => setPublishSourceType('repo')}
                   className={`flex-1 py-3 px-3 rounded-xl border flex items-center justify-center gap-2 text-xs font-bold transition-all ${publishSourceType === 'repo' ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-500 ring-1 ring-emerald-500/20' : 'bg-surface border-divider text-text-muted hover:border-divider-strong'}`}
                 >
                   <GitBranch size={16}/> Load from Repository
                 </button>
                 <button
                   type="button"
                   onClick={() => setPublishSourceType('zip')}
                   className={`flex-1 py-3 px-3 rounded-xl border flex items-center justify-center gap-2 text-xs font-bold transition-all ${publishSourceType === 'zip' ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-500 ring-1 ring-emerald-500/20' : 'bg-surface border-divider text-text-muted hover:border-divider-strong'}`}
                 >
                   <FolderArchive size={16}/> Upload Artifact (.zip)
                 </button>
               </div>
            </div>

            {publishSourceType === 'repo' && (
              <div className="space-y-1.5 animate-in slide-in-from-top-2 duration-300">
                <label className="text-xs font-bold text-text-main flex items-center gap-1.5">
                  <TerminalSquare size={14} className="text-emerald-500" />
                  Remote Git Repository URL <span className="text-red-500">*</span>
                </label>
                <div className="relative">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted text-sm">$</span>
                  <input 
                    type="text" 
                    value={pluginRepoUrl}
                    onChange={(e) => setPluginRepoUrl(e.target.value)}
                    placeholder="https://github.com/user/plugin-repo.git"
                    className="w-full bg-panel border border-divider rounded-lg pl-7 pr-3 py-2.5 text-[13px] font-mono text-text-main focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-all placeholder:font-sans placeholder:text-zinc-500 shadow-inner"
                  />
                </div>
                <p className="text-[10px] text-text-muted px-1">Ensure the repository contains a valid manifest (plugin.json)</p>
              </div>
            )}

            {publishSourceType === 'zip' && (
              <div className="space-y-1.5 animate-in slide-in-from-top-2 duration-300">
                <label className="text-xs font-bold text-text-main flex items-center gap-1.5">
                  <FolderArchive size={14} className="text-emerald-500" />
                  Compressed Plugin Artifact <span className="text-red-500">*</span>
                </label>
                <div 
                  onDragOver={handleDragOver}
                  onDragLeave={handleDragLeave}
                  onDrop={handleDrop}
                  onClick={() => fileInputRef.current?.click()}
                  className={`w-full h-32 mt-1 rounded-xl border-2 border-dashed flex flex-col items-center justify-center cursor-pointer transition-all ${
                    isDragging 
                      ? 'border-emerald-500 bg-emerald-500/10 scale-[1.02]' 
                      : pluginZipFileName
                        ? 'border-emerald-500/50 bg-emerald-500/5'
                        : 'border-divider hover:border-gray-500 hover:bg-surface-hover bg-panel'
                  }`}
                >
                  <input 
                    type="file" 
                    ref={fileInputRef} 
                    onChange={handleFileChange} 
                    accept=".zip" 
                    className="hidden" 
                  />
                  
                  {pluginZipFileName ? (
                    <div className="flex flex-col items-center">
                      <div className="w-10 h-10 bg-emerald-500 text-white rounded-lg flex items-center justify-center mb-2 shadow-sm">
                        <FolderArchive size={20} />
                      </div>
                      <span className="text-[13px] font-bold text-emerald-500">{pluginZipFileName}</span>
                      <span className="text-[10px] text-text-muted mt-0.5">Click to replace file</span>
                    </div>
                  ) : (
                    <>
                      <Upload size={24} className="text-text-muted mb-2" />
                      <p className="text-[13px] font-bold text-text-main">Click or Drag to Upload</p>
                      <p className="text-[11px] text-text-muted mt-1 uppercase tracking-wider">Supports .zip files only</p>
                    </>
                  )}
                </div>
              </div>
            )}

            <div className="space-y-1.5">
              <label className="text-xs font-bold text-text-main">Plugin Category Taxonomy <span>*</span></label>
              <select 
                value={newPluginCategory}
                onChange={(e) => setNewPluginCategory(e.target.value)}
                className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-all custom-select font-medium"
              >
                 {categories.map((cat, idx) => (
                   <option key={idx} value={cat}>{cat}</option>
                 ))}
                 {!categories.includes('Workflows') && <option value="Workflows">Workflows</option>}
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-bold text-text-main">Description <span className="text-red-500">*</span></label>
              <textarea 
                value={newPluginDesc}
                onChange={(e) => setNewPluginDesc(e.target.value)}
                placeholder="What capabilities does this plugin grant to AI models?"
                rows={3}
                className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-all resize-none placeholder:text-zinc-500"
              />
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <label className="text-xs font-bold text-text-main">Invokation Protocol</label>
                <select 
                  value={newPluginSchemaType}
                  onChange={(e) => setNewPluginSchemaType(e.target.value)}
                  className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 transition-all custom-select"
                >
                  <option value="REST API">REST API</option>
                  <option value="GraphQL">GraphQL</option>
                  <option value="gRPC">gRPC</option>
                  <option value="Docker CLI">Docker CLI</option>
                  <option value="Local Script">Local Script</option>
                </select>
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-bold text-text-main">Auth Requirement</label>
                <select 
                  value={newPluginAuthType}
                  onChange={(e) => setNewPluginAuthType(e.target.value)}
                  className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 transition-all custom-select"
                >
                  <option value="None">None (Public)</option>
                  <option value="OAuth 2.0">OAuth 2.0</option>
                  <option value="Bearer Token">Bearer Token</option>
                  <option value="API Key">API Key</option>
                </select>
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-bold text-text-main">Permissions (Comma Separated)</label>
              <input 
                type="text" 
                value={newPluginPermissions}
                onChange={(e) => setNewPluginPermissions(e.target.value)}
                placeholder="e.g. Read local files, Process Image"
                className="w-full bg-surface border border-divider rounded-lg px-3 py-2.5 text-[13px] text-text-main focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 transition-all placeholder:text-zinc-500 font-mono"
              />
            </div>
          </form>
        </div>

        <div className="px-5 py-4 border-t border-divider bg-canvas flex justify-end gap-3 rounded-b-2xl shrink-0">
          <button 
            type="button"
            onClick={onClose}
            className="px-5 py-2 text-[13px] font-bold text-text-muted hover:text-text-main transition-colors rounded-lg bg-surface hover:bg-surface-hover border border-divider"
          >
            Cancel
          </button>
          <button 
            type="submit" 
            form="publishForm"
            className="px-6 py-2 rounded-lg text-[13px] font-bold bg-emerald-600 text-white hover:bg-emerald-500 shadow-md shadow-emerald-500/20 active:scale-[0.98] transition-all"
          >
            Publish Plugin
          </button>
        </div>
      </div>
    </div>
  );
}
