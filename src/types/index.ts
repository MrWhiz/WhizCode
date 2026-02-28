export interface AgentStep {
    tool: string;
    status: 'running' | 'done' | 'error' | 'awaiting_permission';
    summary: string;
    result?: string;
    iteration?: number;
    command?: string;
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

export interface AISettings {
    provider: AIProvider;
    ollamaModel: string;
    openaiKey: string;
    geminiKey: string;
}
