export interface PluginEndpoint {
  method: string;
  path: string;
  desc: string;
}

export interface PluginItem {
  id: number;
  name: string;
  author: string;
  downloads: string;
  rating: number;
  type: string;
  updated: string;
  desc: string;
  installedAgents: string[];
  icon: any; // We will use a string internally or map it, but for UI we might map icon names -> React components
  schemaType: string;
  authType: string;
  permissions: string[];
  endpoints: PluginEndpoint[];
}

export interface PublishPluginInput {
  name: string;
  author: string;
  category: string;
  desc: string;
  schemaType: string;
  authType: string;
  permissions: string[];
  sourceType: 'repo' | 'zip';
  repoUrl?: string;
  fileName?: string;
}

export interface IPluginsService {
  getPlugins(): Promise<PluginItem[]>;
  getCategories(): Promise<string[]>;
  publishPlugin(input: PublishPluginInput): Promise<PluginItem>;
  installPluginToAgents(pluginId: number, agents: string[]): Promise<void>;
}

export interface SkillItem {
  id: number;
  name: string;
  author: string;
  downloads: string;
  rating: number;
  type: string;
  updated: string;
  desc: string;
  installedAgents: string[];
  icon: string;
  schemaType: string;
  authType: string;
  permissions: string[];
  endpoints: PluginEndpoint[];
}

export interface PublishSkillInput {
  name: string;
  author: string;
  category: string;
  desc: string;
  schemaType: string;
  authType: string;
  permissions: string[];
}

export interface ISkillhubService {
  getSkills(): Promise<SkillItem[]>;
  getCategories(): Promise<string[]>;
  publishSkill(input: PublishSkillInput): Promise<SkillItem>;
  installSkillToAgents(skillId: number, agents: string[]): Promise<void>;
}
