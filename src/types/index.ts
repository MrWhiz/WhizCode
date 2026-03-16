export interface AgentStep {
    tool: string;
    status: 'running' | 'done' | 'error' | 'awaiting_permission';
    summary: string;
    result?: string;
    iteration?: number;
    command?: string;
    logs?: string[];
    data?: any;
    requestId?: string;
}

export interface Message {
    role: 'user' | 'assistant';
    content: string;
    steps?: AgentStep[];
}

export interface FileEntry {
    name: string;
    isDirectory: boolean;
    path: string;
}

export interface OpenFileProps {
    name: string;
    path: string;
    content: string;
}

export type AIProvider = 'ollama' | 'openai' | 'gemini';

export interface ModelConfig {
    provider: AIProvider;
    model: string;
}

export interface AISettings {
    planner: ModelConfig;
    executor: ModelConfig;
    openaiKey: string;
    geminiKey: string;
}
