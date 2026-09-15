import { ISystemService, SystemSettings } from './interface';

export * from './interface';

let mockSettings: SystemSettings = {
  // General
  // Left empty on purpose: the previous default pinned one author's macOS home
  // directory (a `/Users/<name>/...` path) into a shipped settings mock, so the
  // config form opened pointing at a directory that exists on no other machine.
  // The user picks a real workspace directory here.
  workspaceDir: '',
  debugLevel: 'info',
  telemetryEnabled: true,
  autoUpdate: true,

  // Security
  masterOverrideKey: '',
  localEncryption: 'aes-256-gcm',
  sessionTimeoutMinutes: 120,

  // Network
  proxyEnabled: false,
  proxyProtocol: 'socks5',
  proxyAddress: '127.0.0.1:7890',
  sslVerification: true,

  // Engine Limits
  maxConcurrentJobs: 4,
  contextWindowTokens: '32k',
  semanticCache: false,
  exactMatchCache: true,

  // UI
  language: 'en',
  theme: 'dark'
};

export class LocalSystemService implements ISystemService {
  async fetchSettings(): Promise<SystemSettings> {
    return new Promise((resolve) => setTimeout(() => resolve({ ...mockSettings }), 300));
  }

  async updateSettings(data: Partial<SystemSettings>): Promise<void> {
    mockSettings = { ...mockSettings, ...data };
    return new Promise((resolve) => setTimeout(resolve, 300));
  }

  async pingLocalCompute(): Promise<boolean> {
    return new Promise((resolve) => setTimeout(() => resolve(true), 1000));
  }

  async configureDatabaseConnection(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 500));
  }

  async setupVectorStore(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 500));
  }

  async runDatabaseMigrations(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 800));
  }

  async clearCache(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 400));
  }
  
  async loadModelToRam(modelName: string): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 1500));
  }
}

export const SystemSettingsService = new LocalSystemService();
