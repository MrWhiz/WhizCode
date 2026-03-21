// Enhanced Code Intelligence System for WhizCode
// Implements deep semantic code analysis and understanding

import * as fs from 'fs';
import * as path from 'path';

export interface CodeSymbol {
  name: string;
  type: 'function' | 'class' | 'variable' | 'interface' | 'type' | 'enum';
  location: {
    file: string;
    line: number;
    column: number;
  };
  scope: string;
  signature?: string;
  documentation?: string;
  dependencies: string[];
  usages: CodeReference[];
}

export interface CodeReference {
  file: string;
  line: number;
  column: number;
  context: string;
  type: 'definition' | 'usage' | 'import' | 'export';
}

export interface CodeRelationship {
  from: string;
  to: string;
  type: 'imports' | 'extends' | 'implements' | 'calls' | 'references';
  strength: number;
}

export interface CodePattern {
  id: string;
  name: string;
  pattern: RegExp;
  description: string;
  category: 'design-pattern' | 'anti-pattern' | 'best-practice' | 'code-smell';
  severity: 'info' | 'warning' | 'error';
  suggestion?: string;
}

export interface CodeMetrics {
  complexity: number;
  maintainability: number;
  testability: number;
  coupling: number;
  cohesion: number;
  linesOfCode: number;
  technicalDebt: number;
}

export interface SemanticContext {
  currentFile: string;
  symbols: Map<string, CodeSymbol>;
  relationships: CodeRelationship[];
  patterns: CodePattern[];
  metrics: CodeMetrics;
  suggestions: string[];
}

export class CodeIntelligence {
  private symbolIndex: Map<string, CodeSymbol> = new Map();
  private relationshipGraph: Map<string, CodeRelationship[]> = new Map();
  private patternLibrary: CodePattern[] = [];
  private workspaceContext: Map<string, SemanticContext> = new Map();

  constructor() {
    this.initializePatternLibrary();
  }

  private initializePatternLibrary() {
    this.patternLibrary = [
      {
        id: 'singleton-pattern',
        name: 'Singleton Pattern',
        pattern: /class\s+\w+\s*{[\s\S]*?private\s+static\s+\w+[\s\S]*?getInstance\(\)/,
        description: 'Singleton design pattern detected',
        category: 'design-pattern',
        severity: 'info',
        suggestion: 'Consider dependency injection for better testability'
      },
      {
        id: 'long-method',
        name: 'Long Method',
        pattern: /function\s+\w+\([^)]*\)\s*{([\s\S]*?)}/,
        description: 'Method is too long and complex',
        category: 'code-smell',
        severity: 'warning',
        suggestion: 'Break down into smaller, focused methods'
      },
      {
        id: 'magic-numbers',
        name: 'Magic Numbers',
        pattern: /(?<![a-zA-Z_$])\d{2,}(?![a-zA-Z_$])/,
        description: 'Magic numbers should be replaced with named constants',
        category: 'code-smell',
        severity: 'warning',
        suggestion: 'Extract magic numbers into named constants'
      },
      {
        id: 'unused-imports',
        name: 'Unused Imports',
        pattern: /import\s+.*?from\s+['"][^'"]+['"]/,
        description: 'Potentially unused import detected',
        category: 'code-smell',
        severity: 'info',
        suggestion: 'Remove unused imports to reduce bundle size'
      }
    ];
  }
  async analyzeWorkspace(workspacePath: string): Promise<SemanticContext> {
    const context: SemanticContext = {
      currentFile: '',
      symbols: new Map(),
      relationships: [],
      patterns: [],
      metrics: this.initializeMetrics(),
      suggestions: []
    };

    try {
      await this.indexWorkspaceSymbols(workspacePath, context);
      await this.analyzeRelationships(workspacePath, context);
      await this.detectPatterns(workspacePath, context);
      context.metrics = await this.calculateMetrics(workspacePath, context);
      context.suggestions = await this.generateSuggestions(context);
    } catch (error) {
      console.error('Error analyzing workspace:', error);
    }

    this.workspaceContext.set(workspacePath, context);
    return context;
  }

  private async indexWorkspaceSymbols(workspacePath: string, context: SemanticContext) {
    const files = await this.findCodeFiles(workspacePath);
    
    for (const file of files) {
      try {
        const content = fs.readFileSync(file, 'utf8');
        const symbols = await this.extractSymbols(file, content);
        
        symbols.forEach(symbol => {
          context.symbols.set(`${file}:${symbol.name}`, symbol);
          this.symbolIndex.set(`${file}:${symbol.name}`, symbol);
        });
      } catch (error) {
        console.warn(`Failed to index symbols in ${file}:`, error);
      }
    }
  }

  private async findCodeFiles(workspacePath: string): Promise<string[]> {
    const codeFiles: string[] = [];
    const codeExtensions = ['.ts', '.tsx', '.js', '.jsx', '.py', '.java', '.cpp', '.c', '.cs', '.go', '.rs'];

    const walkDir = (dir: string, depth: number = 0) => {
      if (depth > 5) return; // Limit recursion depth

      try {
        const files = fs.readdirSync(dir);
        
        for (const file of files) {
          const filePath = path.join(dir, file);
          const stat = fs.statSync(filePath);

          if (stat.isDirectory()) {
            if (!file.startsWith('.') && file !== 'node_modules' && file !== 'dist') {
              walkDir(filePath, depth + 1);
            }
          } else if (stat.isFile()) {
            const ext = path.extname(file).toLowerCase();
            if (codeExtensions.includes(ext)) {
              codeFiles.push(filePath);
            }
          }
        }
      } catch (error) {
        console.warn(`Failed to read directory ${dir}:`, error);
      }
    };

    walkDir(workspacePath);
    return codeFiles;
  }

  private async extractSymbols(filePath: string, content: string): Promise<CodeSymbol[]> {
    const symbols: CodeSymbol[] = [];
    const lines = content.split('\n');

    // Extract functions
    const functionRegex = /(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\([^)]*\)/g;
    let match;
    
    while ((match = functionRegex.exec(content)) !== null) {
      const lineNumber = content.substring(0, match.index).split('\n').length;
      symbols.push({
        name: match[1],
        type: 'function',
        location: { file: filePath, line: lineNumber, column: match.index },
        scope: this.determineScope(content, match.index),
        signature: match[0],
        dependencies: this.extractDependencies(content, match.index),
        usages: []
      });
    }

    // Extract classes
    const classRegex = /(?:export\s+)?class\s+(\w+)(?:\s+extends\s+\w+)?(?:\s+implements\s+[\w,\s]+)?\s*{/g;
    
    while ((match = classRegex.exec(content)) !== null) {
      const lineNumber = content.substring(0, match.index).split('\n').length;
      symbols.push({
        name: match[1],
        type: 'class',
        location: { file: filePath, line: lineNumber, column: match.index },
        scope: 'global',
        signature: match[0],
        dependencies: this.extractDependencies(content, match.index),
        usages: []
      });
    }

    // Extract interfaces
    const interfaceRegex = /(?:export\s+)?interface\s+(\w+)(?:\s+extends\s+[\w,\s]+)?\s*{/g;
    
    while ((match = interfaceRegex.exec(content)) !== null) {
      const lineNumber = content.substring(0, match.index).split('\n').length;
      symbols.push({
        name: match[1],
        type: 'interface',
        location: { file: filePath, line: lineNumber, column: match.index },
        scope: 'global',
        signature: match[0],
        dependencies: [],
        usages: []
      });
    }

    // Extract variables and constants
    const variableRegex = /(?:export\s+)?(?:const|let|var)\s+(\w+)(?:\s*:\s*\w+)?\s*=/g;
    
    while ((match = variableRegex.exec(content)) !== null) {
      const lineNumber = content.substring(0, match.index).split('\n').length;
      symbols.push({
        name: match[1],
        type: 'variable',
        location: { file: filePath, line: lineNumber, column: match.index },
        scope: this.determineScope(content, match.index),
        signature: match[0],
        dependencies: [],
        usages: []
      });
    }

    return symbols;
  }

  private determineScope(content: string, position: number): string {
    const beforePosition = content.substring(0, position);
    const functionMatches = beforePosition.match(/function\s+\w+/g) || [];
    const classMatches = beforePosition.match(/class\s+\w+/g) || [];
    
    if (functionMatches.length > classMatches.length) {
      return 'function';
    } else if (classMatches.length > 0) {
      return 'class';
    }
    
    return 'global';
  }

  private extractDependencies(content: string, position: number): string[] {
    const dependencies: string[] = [];
    
    // Look for imports at the top of the file
    const importRegex = /import\s+.*?from\s+['"]([^'"]+)['"]/g;
    let match;
    
    while ((match = importRegex.exec(content)) !== null) {
      if (match.index < position) {
        dependencies.push(match[1]);
      }
    }
    
    return dependencies;
  }

  private async analyzeRelationships(workspacePath: string, context: SemanticContext) {
    const relationships: CodeRelationship[] = [];
    
    context.symbols.forEach((symbol, key) => {
      // Analyze imports
      symbol.dependencies.forEach(dep => {
        relationships.push({
          from: symbol.location.file,
          to: dep,
          type: 'imports',
          strength: 1.0
        });
      });
      
      // Find usages of this symbol in other files
      this.findSymbolUsages(symbol, context).forEach(usage => {
        relationships.push({
          from: usage.file,
          to: symbol.location.file,
          type: 'references',
          strength: 0.5
        });
      });
    });
    
    context.relationships = relationships;
    this.relationshipGraph.set(workspacePath, relationships);
  }

  private findSymbolUsages(symbol: CodeSymbol, context: SemanticContext): CodeReference[] {
    const usages: CodeReference[] = [];
    
    context.symbols.forEach((otherSymbol, key) => {
      if (otherSymbol.location.file !== symbol.location.file) {
        try {
          const content = fs.readFileSync(otherSymbol.location.file, 'utf8');
          const escapedSymbolName = symbol.name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
          const regex = new RegExp(`\\b${escapedSymbolName}\\b`, 'g');
          let match;
          
          while ((match = regex.exec(content)) !== null) {
            const lineNumber = content.substring(0, match.index).split('\n').length;
            usages.push({
              file: otherSymbol.location.file,
              line: lineNumber,
              column: match.index,
              context: this.getLineContext(content, lineNumber),
              type: 'usage'
            });
          }
        } catch (error) {
          // Ignore file read errors
        }
      }
    });
    
    return usages;
  }

  private getLineContext(content: string, lineNumber: number): string {
    const lines = content.split('\n');
    return lines[lineNumber - 1] || '';
  }

  private async detectPatterns(workspacePath: string, context: SemanticContext) {
    const detectedPatterns: CodePattern[] = [];
    const files = await this.findCodeFiles(workspacePath);
    
    for (const file of files) {
      try {
        const content = fs.readFileSync(file, 'utf8');
        
        for (const pattern of this.patternLibrary) {
          const matches = content.match(pattern.pattern);
          if (matches) {
            detectedPatterns.push({
              ...pattern,
              id: `${pattern.id}_${file}_${Date.now()}`
            });
          }
        }
      } catch (error) {
        console.warn(`Failed to analyze patterns in ${file}:`, error);
      }
    }
    
    context.patterns = detectedPatterns;
  }

  private initializeMetrics(): CodeMetrics {
    return {
      complexity: 0,
      maintainability: 0,
      testability: 0,
      coupling: 0,
      cohesion: 0,
      linesOfCode: 0,
      technicalDebt: 0
    };
  }

  private async calculateMetrics(workspacePath: string, context: SemanticContext): Promise<CodeMetrics> {
    const files = await this.findCodeFiles(workspacePath);
    let totalLines = 0;
    let totalComplexity = 0;
    let totalCoupling = 0;
    
    for (const file of files) {
      try {
        const content = fs.readFileSync(file, 'utf8');
        const lines = content.split('\n').length;
        totalLines += lines;
        
        // Calculate cyclomatic complexity (simplified)
        const complexity = this.calculateCyclomaticComplexity(content);
        totalComplexity += complexity;
        
        // Calculate coupling (number of imports)
        const imports = (content.match(/import\s+.*?from/g) || []).length;
        totalCoupling += imports;
      } catch (error) {
        console.warn(`Failed to calculate metrics for ${file}:`, error);
      }
    }
    
    const fileCount = files.length;
    const avgComplexity = fileCount > 0 ? totalComplexity / fileCount : 0;
    const avgCoupling = fileCount > 0 ? totalCoupling / fileCount : 0;
    
    return {
      complexity: avgComplexity,
      maintainability: Math.max(0, 100 - avgComplexity * 2 - avgCoupling),
      testability: Math.max(0, 100 - avgComplexity - avgCoupling * 2),
      coupling: avgCoupling,
      cohesion: this.calculateCohesion(context),
      linesOfCode: totalLines,
      technicalDebt: this.calculateTechnicalDebt(context)
    };
  }

  private calculateCyclomaticComplexity(content: string): number {
    // Simplified cyclomatic complexity calculation
    const wordKeywords = ['if', 'else', 'while', 'for', 'switch', 'case', 'catch'];
    const operatorKeywords = ['&&', '||', '?'];
    let complexity = 1; // Base complexity
    
    // Count word-bounded keywords
    for (const keyword of wordKeywords) {
      const regex = new RegExp(`\\b${keyword}\\b`, 'g');
      const matches = content.match(regex);
      if (matches) {
        complexity += matches.length;
      }
    }
    
    // Count operator keywords (without word boundaries)
    for (const operator of operatorKeywords) {
      const escapedOperator = operator.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp(escapedOperator, 'g');
      const matches = content.match(regex);
      if (matches) {
        complexity += matches.length;
      }
    }
    
    return complexity;
  }

  private calculateCohesion(context: SemanticContext): number {
    // Simplified cohesion calculation based on symbol relationships
    const totalSymbols = context.symbols.size;
    const totalRelationships = context.relationships.length;
    
    if (totalSymbols === 0) return 0;
    
    return Math.min(100, (totalRelationships / totalSymbols) * 20);
  }

  private calculateTechnicalDebt(context: SemanticContext): number {
    // Calculate technical debt based on code smells and anti-patterns
    let debtScore = 0;
    
    context.patterns.forEach(pattern => {
      switch (pattern.severity) {
        case 'error':
          debtScore += 10;
          break;
        case 'warning':
          debtScore += 5;
          break;
        case 'info':
          debtScore += 1;
          break;
      }
    });
    
    return debtScore;
  }

  private async generateSuggestions(context: SemanticContext): Promise<string[]> {
    const suggestions: string[] = [];
    
    // Suggestions based on metrics
    if (context.metrics.complexity > 10) {
      suggestions.push('Consider breaking down complex functions into smaller, more focused units');
    }
    
    if (context.metrics.coupling > 15) {
      suggestions.push('High coupling detected - consider dependency injection or facade patterns');
    }
    
    if (context.metrics.cohesion < 30) {
      suggestions.push('Low cohesion detected - group related functionality together');
    }
    
    // Suggestions based on patterns
    const codeSmells = context.patterns.filter(p => p.category === 'code-smell');
    if (codeSmells.length > 5) {
      suggestions.push('Multiple code smells detected - consider refactoring for better maintainability');
    }
    
    const antiPatterns = context.patterns.filter(p => p.category === 'anti-pattern');
    if (antiPatterns.length > 0) {
      suggestions.push('Anti-patterns detected - review and refactor problematic code');
    }
    
    // Suggestions based on symbol analysis
    const unusedSymbols = this.findUnusedSymbols(context);
    if (unusedSymbols.length > 0) {
      suggestions.push(`${unusedSymbols.length} potentially unused symbols found - consider cleanup`);
    }
    
    return suggestions;
  }

  private findUnusedSymbols(context: SemanticContext): CodeSymbol[] {
    const unusedSymbols: CodeSymbol[] = [];
    
    context.symbols.forEach(symbol => {
      if (symbol.usages.length === 0 && !symbol.name.startsWith('_')) {
        unusedSymbols.push(symbol);
      }
    });
    
    return unusedSymbols;
  }

  // Public API methods
  async getSymbolInfo(workspacePath: string, symbolName: string): Promise<CodeSymbol | null> {
    const context = this.workspaceContext.get(workspacePath);
    if (!context) return null;
    
    for (const [key, symbol] of context.symbols) {
      if (symbol.name === symbolName) {
        return symbol;
      }
    }
    
    return null;
  }

  async findRelatedSymbols(workspacePath: string, symbolName: string): Promise<CodeSymbol[]> {
    const context = this.workspaceContext.get(workspacePath);
    if (!context) return [];
    
    const relatedSymbols: CodeSymbol[] = [];
    const targetSymbol = await this.getSymbolInfo(workspacePath, symbolName);
    
    if (!targetSymbol) return [];
    
    // Find symbols in the same file
    context.symbols.forEach(symbol => {
      if (symbol.location.file === targetSymbol.location.file && symbol.name !== symbolName) {
        relatedSymbols.push(symbol);
      }
    });
    
    // Find symbols that reference this symbol
    context.relationships.forEach(rel => {
      if (rel.to === targetSymbol.location.file) {
        context.symbols.forEach(symbol => {
          if (symbol.location.file === rel.from && !relatedSymbols.includes(symbol)) {
            relatedSymbols.push(symbol);
          }
        });
      }
    });
    
    return relatedSymbols;
  }

  async suggestRefactoring(workspacePath: string, filePath: string): Promise<string[]> {
    const context = this.workspaceContext.get(workspacePath);
    if (!context) return [];
    
    const suggestions: string[] = [];
    
    // Find symbols in the target file
    const fileSymbols = Array.from(context.symbols.values())
      .filter(symbol => symbol.location.file === filePath);
    
    // Analyze each symbol for refactoring opportunities
    for (const symbol of fileSymbols) {
      if (symbol.type === 'function') {
        try {
          const content = fs.readFileSync(filePath, 'utf8');
          const complexity = this.calculateCyclomaticComplexity(content);
          
          if (complexity > 10) {
            suggestions.push(`Function '${symbol.name}' has high complexity (${complexity}) - consider breaking it down`);
          }
        } catch (error) {
          // Ignore file read errors
        }
      }
      
      if (symbol.usages.length === 0) {
        suggestions.push(`Symbol '${symbol.name}' appears to be unused - consider removing it`);
      }
    }
    
    return suggestions;
  }

  getWorkspaceContext(workspacePath: string): SemanticContext | undefined {
    return this.workspaceContext.get(workspacePath);
  }

  getAllSymbols(workspacePath: string): CodeSymbol[] {
    const context = this.workspaceContext.get(workspacePath);
    return context ? Array.from(context.symbols.values()) : [];
  }

  getCodeMetrics(workspacePath: string): CodeMetrics | null {
    const context = this.workspaceContext.get(workspacePath);
    return context ? context.metrics : null;
  }
}