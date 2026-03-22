// Context Memory System for WhizCode
// Implements persistent context management and learning from interactions

import * as fs from 'fs';
import * as path from 'path';
import { app } from 'electron';

export interface CodePattern {
  id: string;
  pattern: string;
  context: string;
  frequency: number;
  lastSeen: Date;
  projectType?: string;
  language?: string;
}

export interface UserPreference {
  key: string;
  value: any;
  confidence: number;
  lastUpdated: Date;
}

export interface ErrorPattern {
  id: string;
  errorType: string;
  context: string;
  solution: string;
  successRate: number;
  occurrences: number;
  lastOccurrence: Date;
}

export interface SuccessfulStrategy {
  id: string;
  taskType: string;
  strategy: string;
  tools: string[];
  successRate: number;
  averageDuration: number;
  usageCount: number;
  lastUsed: Date;
}

export interface ProjectContext {
  workspacePath: string;
  projectType: string;
  languages: string[];
  frameworks: string[];
  patterns: CodePattern[];
  commonFiles: string[];
  lastAnalyzed: Date;
  packageStatus?: {
    hasPackageJson: boolean;
    hasNodeModules: boolean;
    installedPackages: string[];
    missingPackages: string[];
    lastChecked: Date;
  };
}

export interface SessionMemory {
  sessionId: string;
  startTime: Date;
  endTime?: Date;
  interactions: InteractionMemory[];
  outcomes: string[];
  userSatisfaction?: number;
}

export interface InteractionMemory {
  timestamp: Date;
  userRequest: string;
  agentResponse: string;
  toolsUsed: string[];
  success: boolean;
  duration: number;
  context: any;
}

export class ContextMemory {
  private memoryPath: string;
  private codePatterns: Map<string, CodePattern> = new Map();
  private userPreferences: Map<string, UserPreference> = new Map();
  private errorPatterns: Map<string, ErrorPattern> = new Map();
  private successfulStrategies: Map<string, SuccessfulStrategy> = new Map();
  private projectContexts: Map<string, ProjectContext> = new Map();
  private sessionHistory: SessionMemory[] = [];
  private currentSession?: SessionMemory;

  constructor() {
    this.memoryPath = path.join(app.getPath('userData'), 'context-memory');
    this.ensureMemoryDirectory();
    this.loadMemory();
  }

  private ensureMemoryDirectory() {
    if (!fs.existsSync(this.memoryPath)) {
      fs.mkdirSync(this.memoryPath, { recursive: true });
    }
  }

  private async loadMemory() {
    try {
      await Promise.all([
        this.loadCodePatterns(),
        this.loadUserPreferences(),
        this.loadErrorPatterns(),
        this.loadSuccessfulStrategies(),
        this.loadProjectContexts(),
        this.loadSessionHistory()
      ]);
    } catch (error) {
      console.warn('Failed to load some memory components:', error);
    }
  }

  private async loadCodePatterns() {
    const filePath = path.join(this.memoryPath, 'code-patterns.json');
    if (fs.existsSync(filePath)) {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      data.forEach((pattern: any) => {
        pattern.lastSeen = new Date(pattern.lastSeen);
        this.codePatterns.set(pattern.id, pattern);
      });
    }
  }

  private async loadUserPreferences() {
    const filePath = path.join(this.memoryPath, 'user-preferences.json');
    if (fs.existsSync(filePath)) {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      data.forEach((pref: any) => {
        pref.lastUpdated = new Date(pref.lastUpdated);
        this.userPreferences.set(pref.key, pref);
      });
    }
  }

  private async loadErrorPatterns() {
    const filePath = path.join(this.memoryPath, 'error-patterns.json');
    if (fs.existsSync(filePath)) {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      data.forEach((error: any) => {
        error.lastOccurrence = new Date(error.lastOccurrence);
        this.errorPatterns.set(error.id, error);
      });
    }
  }

  private async loadSuccessfulStrategies() {
    const filePath = path.join(this.memoryPath, 'successful-strategies.json');
    if (fs.existsSync(filePath)) {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      data.forEach((strategy: any) => {
        strategy.lastUsed = new Date(strategy.lastUsed);
        this.successfulStrategies.set(strategy.id, strategy);
      });
    }
  }

  private async loadProjectContexts() {
    const filePath = path.join(this.memoryPath, 'project-contexts.json');
    if (fs.existsSync(filePath)) {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      data.forEach((context: any) => {
        context.lastAnalyzed = new Date(context.lastAnalyzed);
        context.patterns.forEach((p: any) => p.lastSeen = new Date(p.lastSeen));
        this.projectContexts.set(context.workspacePath, context);
      });
    }
  }

  private async loadSessionHistory() {
    const filePath = path.join(this.memoryPath, 'session-history.json');
    if (fs.existsSync(filePath)) {
      const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      this.sessionHistory = data.map((session: any) => ({
        ...session,
        startTime: new Date(session.startTime),
        endTime: session.endTime ? new Date(session.endTime) : undefined,
        interactions: session.interactions.map((i: any) => ({
          ...i,
          timestamp: new Date(i.timestamp)
        }))
      }));
    }
  }

  async saveMemory() {
    try {
      await Promise.all([
        this.saveCodePatterns(),
        this.saveUserPreferences(),
        this.saveErrorPatterns(),
        this.saveSuccessfulStrategies(),
        this.saveProjectContexts(),
        this.saveSessionHistory()
      ]);
    } catch (error) {
      console.error('Failed to save memory:', error);
    }
  }

  private async saveCodePatterns() {
    const filePath = path.join(this.memoryPath, 'code-patterns.json');
    const data = Array.from(this.codePatterns.values());
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  }

  private async saveUserPreferences() {
    const filePath = path.join(this.memoryPath, 'user-preferences.json');
    const data = Array.from(this.userPreferences.values());
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  }

  private async saveErrorPatterns() {
    const filePath = path.join(this.memoryPath, 'error-patterns.json');
    const data = Array.from(this.errorPatterns.values());
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  }

  private async saveSuccessfulStrategies() {
    const filePath = path.join(this.memoryPath, 'successful-strategies.json');
    const data = Array.from(this.successfulStrategies.values());
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  }

  private async saveProjectContexts() {
    const filePath = path.join(this.memoryPath, 'project-contexts.json');
    const data = Array.from(this.projectContexts.values());
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
  }

  private async saveSessionHistory() {
    const filePath = path.join(this.memoryPath, 'session-history.json');
    // Keep only last 100 sessions to prevent unbounded growth
    const recentSessions = this.sessionHistory.slice(-100);
    fs.writeFileSync(filePath, JSON.stringify(recentSessions, null, 2));
  }

  // Code Pattern Management
  recordCodePattern(pattern: string, context: string, projectType?: string, language?: string) {
    const id = this.generatePatternId(pattern, context);
    const existing = this.codePatterns.get(id);

    if (existing) {
      existing.frequency++;
      existing.lastSeen = new Date();
    } else {
      this.codePatterns.set(id, {
        id,
        pattern,
        context,
        frequency: 1,
        lastSeen: new Date(),
        projectType,
        language
      });
    }
  }

  getRelevantCodePatterns(context: string, projectType?: string, language?: string): CodePattern[] {
    return Array.from(this.codePatterns.values())
      .filter(p => {
        const contextMatch = p.context.toLowerCase().includes(context.toLowerCase()) ||
                           context.toLowerCase().includes(p.context.toLowerCase());
        const projectMatch = !projectType || !p.projectType || p.projectType === projectType;
        const languageMatch = !language || !p.language || p.language === language;
        return contextMatch && projectMatch && languageMatch;
      })
      .sort((a, b) => b.frequency - a.frequency)
      .slice(0, 10);
  }

  // User Preference Management
  recordUserPreference(key: string, value: any, confidence: number = 1.0) {
    const existing = this.userPreferences.get(key);
    
    if (existing) {
      // Update with weighted average
      const totalWeight = existing.confidence + confidence;
      existing.value = this.mergePreferenceValues(existing.value, value, existing.confidence / totalWeight);
      existing.confidence = Math.min(totalWeight, 10); // Cap confidence
      existing.lastUpdated = new Date();
    } else {
      this.userPreferences.set(key, {
        key,
        value,
        confidence,
        lastUpdated: new Date()
      });
    }
  }

  getUserPreference(key: string): any {
    const pref = this.userPreferences.get(key);
    return pref?.value;
  }

  // Error Pattern Management
  recordErrorPattern(errorType: string, context: string, solution: string, success: boolean) {
    const id = this.generateErrorId(errorType, context);
    const existing = this.errorPatterns.get(id);

    if (existing) {
      existing.occurrences++;
      existing.lastOccurrence = new Date();
      if (success) {
        existing.successRate = (existing.successRate * (existing.occurrences - 1) + 1) / existing.occurrences;
        existing.solution = solution; // Update with latest successful solution
      } else {
        existing.successRate = (existing.successRate * (existing.occurrences - 1)) / existing.occurrences;
      }
    } else {
      this.errorPatterns.set(id, {
        id,
        errorType,
        context,
        solution,
        successRate: success ? 1.0 : 0.0,
        occurrences: 1,
        lastOccurrence: new Date()
      });
    }
  }

  getSimilarErrorPatterns(errorType: string, context: string): ErrorPattern[] {
    return Array.from(this.errorPatterns.values())
      .filter(p => {
        const typeMatch = p.errorType.toLowerCase().includes(errorType.toLowerCase()) ||
                        errorType.toLowerCase().includes(p.errorType.toLowerCase());
        const contextMatch = p.context.toLowerCase().includes(context.toLowerCase()) ||
                           context.toLowerCase().includes(p.context.toLowerCase());
        return typeMatch || contextMatch;
      })
      .sort((a, b) => b.successRate - a.successRate)
      .slice(0, 5);
  }

  // Strategy Management
  recordSuccessfulStrategy(taskType: string, strategy: string, tools: string[], duration: number, success: boolean) {
    const id = this.generateStrategyId(taskType, strategy);
    const existing = this.successfulStrategies.get(id);

    if (existing) {
      existing.usageCount++;
      existing.lastUsed = new Date();
      existing.averageDuration = (existing.averageDuration * (existing.usageCount - 1) + duration) / existing.usageCount;
      
      if (success) {
        existing.successRate = (existing.successRate * (existing.usageCount - 1) + 1) / existing.usageCount;
      } else {
        existing.successRate = (existing.successRate * (existing.usageCount - 1)) / existing.usageCount;
      }
    } else {
      this.successfulStrategies.set(id, {
        id,
        taskType,
        strategy,
        tools,
        successRate: success ? 1.0 : 0.0,
        averageDuration: duration,
        usageCount: 1,
        lastUsed: new Date()
      });
    }
  }

  getBestStrategies(taskType: string): SuccessfulStrategy[] {
    return Array.from(this.successfulStrategies.values())
      .filter(s => s.taskType.toLowerCase().includes(taskType.toLowerCase()) ||
                  taskType.toLowerCase().includes(s.taskType.toLowerCase()))
      .sort((a, b) => b.successRate - a.successRate)
      .slice(0, 3);
  }

  // Project Context Management
  async analyzeProjectContext(workspacePath: string): Promise<ProjectContext> {
    const existing = this.projectContexts.get(workspacePath);
    const now = new Date();
    
    // Return cached context if analyzed recently (within 1 hour for better responsiveness)
    if (existing && (now.getTime() - existing.lastAnalyzed.getTime()) < 60 * 60 * 1000) {
      return existing;
    }

    // Analyze project structure
    const context: ProjectContext = {
      workspacePath,
      projectType: await this.detectProjectType(workspacePath),
      languages: await this.detectLanguages(workspacePath),
      frameworks: await this.detectFrameworks(workspacePath),
      patterns: existing?.patterns || [],
      commonFiles: await this.findCommonFiles(workspacePath),
      lastAnalyzed: now
    };

    // Add package installation status
    await this.checkPackageStatus(context);

    this.projectContexts.set(workspacePath, context);
    await this.saveProjectContexts(); // Persist immediately
    return context;
  }

  // Session Management
  startSession(): string {
    const sessionId = `session_${Date.now()}`;
    this.currentSession = {
      sessionId,
      startTime: new Date(),
      interactions: [],
      outcomes: []
    };
    return sessionId;
  }

  recordInteraction(userRequest: string, agentResponse: string, toolsUsed: string[], success: boolean, duration: number, context?: any) {
    if (!this.currentSession) return;

    this.currentSession.interactions.push({
      timestamp: new Date(),
      userRequest,
      agentResponse,
      toolsUsed,
      success,
      duration,
      context
    });
  }

  endSession(userSatisfaction?: number) {
    if (!this.currentSession) return;

    this.currentSession.endTime = new Date();
    this.currentSession.userSatisfaction = userSatisfaction;
    
    this.sessionHistory.push(this.currentSession);
    this.currentSession = undefined;
  }

  // Helper methods
  private generatePatternId(pattern: string, context: string): string {
    return `pattern_${this.hashString(pattern + context)}`;
  }

  private generateErrorId(errorType: string, context: string): string {
    return `error_${this.hashString(errorType + context)}`;
  }

  private generateStrategyId(taskType: string, strategy: string): string {
    return `strategy_${this.hashString(taskType + strategy)}`;
  }

  private hashString(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(36);
  }

  private mergePreferenceValues(existing: any, newValue: any, existingWeight: number): any {
    if (typeof existing === 'number' && typeof newValue === 'number') {
      return existing * existingWeight + newValue * (1 - existingWeight);
    }
    return newValue; // For non-numeric values, use the new value
  }

  private async detectProjectType(workspacePath: string): Promise<string> {
    // Simple project type detection based on files
    try {
      const files = fs.readdirSync(workspacePath);
      
      if (files.includes('package.json')) {
        const packageJson = JSON.parse(fs.readFileSync(path.join(workspacePath, 'package.json'), 'utf8'));
        if (packageJson.dependencies?.react) return 'react';
        if (packageJson.dependencies?.vue) return 'vue';
        if (packageJson.dependencies?.angular) return 'angular';
        if (packageJson.dependencies?.electron) return 'electron';
        return 'nodejs';
      }
      
      if (files.includes('requirements.txt') || files.includes('setup.py')) return 'python';
      if (files.includes('Cargo.toml')) return 'rust';
      if (files.includes('go.mod')) return 'go';
      if (files.includes('pom.xml') || files.includes('build.gradle')) return 'java';
      
      return 'unknown';
    } catch {
      return 'unknown';
    }
  }

  private async detectLanguages(workspacePath: string): Promise<string[]> {
    const languages = new Set<string>();
    
    try {
      const walkDir = (dir: string, depth: number = 0) => {
        if (depth > 3) return; // Limit recursion depth
        
        const files = fs.readdirSync(dir);
        for (const file of files) {
          const filePath = path.join(dir, file);
          const stat = fs.statSync(filePath);
          
          if (stat.isDirectory() && !file.startsWith('.') && file !== 'node_modules') {
            walkDir(filePath, depth + 1);
          } else if (stat.isFile()) {
            const ext = path.extname(file).toLowerCase();
            const langMap: Record<string, string> = {
              '.js': 'javascript',
              '.ts': 'typescript',
              '.jsx': 'javascript',
              '.tsx': 'typescript',
              '.py': 'python',
              '.rs': 'rust',
              '.go': 'go',
              '.java': 'java',
              '.cpp': 'cpp',
              '.c': 'c',
              '.cs': 'csharp',
              '.php': 'php',
              '.rb': 'ruby'
            };
            
            if (langMap[ext]) {
              languages.add(langMap[ext]);
            }
          }
        }
      };
      
      walkDir(workspacePath);
    } catch {
      // Ignore errors
    }
    
    return Array.from(languages);
  }

  private async detectFrameworks(workspacePath: string): Promise<string[]> {
    const frameworks: string[] = [];
    
    try {
      const packageJsonPath = path.join(workspacePath, 'package.json');
      if (fs.existsSync(packageJsonPath)) {
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
        const deps = { ...packageJson.dependencies, ...packageJson.devDependencies };
        
        const frameworkMap: Record<string, string> = {
          'react': 'React',
          'vue': 'Vue.js',
          '@angular/core': 'Angular',
          'electron': 'Electron',
          'express': 'Express.js',
          'next': 'Next.js',
          'nuxt': 'Nuxt.js',
          'svelte': 'Svelte',
          'vite': 'Vite',
          'webpack': 'Webpack'
        };
        
        for (const [dep, framework] of Object.entries(frameworkMap)) {
          if (deps[dep]) {
            frameworks.push(framework);
          }
        }
      }
    } catch {
      // Ignore errors
    }
    
    return frameworks;
  }

  private async findCommonFiles(workspacePath: string): Promise<string[]> {
    const commonFiles: string[] = [];
    
    try {
      const files = fs.readdirSync(workspacePath);
      const importantFiles = [
        'package.json', 'README.md', 'tsconfig.json', 'vite.config.ts',
        'webpack.config.js', '.gitignore', 'Dockerfile', 'docker-compose.yml'
      ];
      
      for (const file of importantFiles) {
        if (files.includes(file)) {
          commonFiles.push(file);
        }
      }
    } catch {
      // Ignore errors
    }
    
    return commonFiles;
  }

  // Public getters for memory data
  getCodePatterns(): CodePattern[] {
    return Array.from(this.codePatterns.values());
  }

  getUserPreferences(): UserPreference[] {
    return Array.from(this.userPreferences.values());
  }

  getErrorPatterns(): ErrorPattern[] {
    return Array.from(this.errorPatterns.values());
  }

  getSuccessfulStrategies(): SuccessfulStrategy[] {
    return Array.from(this.successfulStrategies.values());
  }

  getProjectContexts(): ProjectContext[] {
    return Array.from(this.projectContexts.values());
  }

  getSessionHistory(): SessionMemory[] {
    return this.sessionHistory;
  }

  private async checkPackageStatus(context: ProjectContext): Promise<void> {
    const packageJsonPath = path.join(context.workspacePath, 'package.json');
    const nodeModulesPath = path.join(context.workspacePath, 'node_modules');
    
    const packageStatus = {
      hasPackageJson: fs.existsSync(packageJsonPath),
      hasNodeModules: fs.existsSync(nodeModulesPath),
      installedPackages: [] as string[],
      missingPackages: [] as string[],
      lastChecked: new Date()
    };

    if (packageStatus.hasPackageJson) {
      try {
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
        const allDeps = {
          ...packageJson.dependencies || {},
          ...packageJson.devDependencies || {}
        };

        for (const [packageName] of Object.entries(allDeps)) {
          const packagePath = path.join(nodeModulesPath, packageName);
          if (fs.existsSync(packagePath)) {
            packageStatus.installedPackages.push(packageName);
          } else {
            packageStatus.missingPackages.push(packageName);
          }
        }
      } catch (error) {
        // Error reading package.json, skip package status
      }
    }

    context.packageStatus = packageStatus;
  }

  getCurrentWorkspaceContext(workspacePath: string): ProjectContext | null {
    return this.projectContexts.get(workspacePath) || null;
  }

  async refreshPackageStatus(workspacePath: string): Promise<void> {
    const context = this.projectContexts.get(workspacePath);
    if (context) {
      await this.checkPackageStatus(context);
      await this.saveProjectContexts();
    }
  }
}