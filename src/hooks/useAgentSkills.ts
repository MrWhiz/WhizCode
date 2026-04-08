import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SelectedSkill {
  name: string;
  confidence: number;
  capabilities: string[];
  context: {
    workspace_path: string;
    query: string;
    project_type: string;
    files: string[];
  };
}

interface SkillSelectionResult {
  selected_skills: SelectedSkill[];
  conflicts_resolved: Array<{
    skill_a: string;
    skill_b: string;
    resolution: string;
    winner: string;
  }>;
}

interface UseAgentSkillsReturn {
  selectSkillsForTask: (
    task: string,
    workspacePath: string | null,
    projectType: string,
    files: string[],
  ) => Promise<SkillSelectionResult>;
  getSkillsContextForPrompt: (
    task: string,
    workspacePath: string | null,
    projectType: string,
    files: string[],
  ) => Promise<string>;
  getSkillsSystemPrompt: (
    task: string,
    workspacePath: string | null,
    projectType: string,
    files: string[],
  ) => Promise<string>;
  getSkillsForUI: (
    task: string,
    workspacePath: string | null,
    projectType: string,
    files: string[],
  ) => Promise<any>;
  loading: boolean;
  error: string | null;
}

/**
 * Custom hook for integrating skills with agent queries
 *
 * Provides functions to select relevant skills for a task and format
 * them for inclusion in the agent's system prompt and UI.
 *
 * @returns {UseAgentSkillsReturn} Skills selection functions and state
 */
export const useAgentSkills = (): UseAgentSkillsReturn => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Select skills for a specific task
   */
  const selectSkillsForTask = useCallback(
    async (
      task: string,
      workspacePath: string | null,
      projectType: string,
      files: string[],
    ): Promise<SkillSelectionResult> => {
      try {
        setLoading(true);
        setError(null);

        const result = await invoke<SkillSelectionResult>(
          "select_skills_for_task",
          {
            task,
            workspace_path: workspacePath,
            project_type: projectType,
            files,
          },
        );

        return result;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`Failed to select skills: ${errorMessage}`);
        console.error("Error selecting skills:", err);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  /**
   * Get formatted skills context for agent prompt
   */
  const getSkillsContextForPrompt = useCallback(
    async (
      task: string,
      workspacePath: string | null,
      projectType: string,
      files: string[],
    ): Promise<string> => {
      try {
        setLoading(true);
        setError(null);

        const context = await invoke<string>("get_skills_context_for_prompt", {
          task,
          workspace_path: workspacePath,
          project_type: projectType,
          files,
        });

        return context;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`Failed to get skills context: ${errorMessage}`);
        console.error("Error getting skills context:", err);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  /**
   * Get skills system prompt addition
   */
  const getSkillsSystemPrompt = useCallback(
    async (
      task: string,
      workspacePath: string | null,
      projectType: string,
      files: string[],
    ): Promise<string> => {
      try {
        setLoading(true);
        setError(null);

        const prompt = await invoke<string>("get_skills_system_prompt", {
          task,
          workspace_path: workspacePath,
          project_type: projectType,
          files,
        });

        return prompt;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`Failed to get skills system prompt: ${errorMessage}`);
        console.error("Error getting skills system prompt:", err);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  /**
   * Get skills formatted for UI display
   */
  const getSkillsForUI = useCallback(
    async (
      task: string,
      workspacePath: string | null,
      projectType: string,
      files: string[],
    ): Promise<any> => {
      try {
        setLoading(true);
        setError(null);

        const skillsUI = await invoke<any>("get_skills_for_ui", {
          task,
          workspace_path: workspacePath,
          project_type: projectType,
          files,
        });

        return skillsUI;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setError(`Failed to get skills for UI: ${errorMessage}`);
        console.error("Error getting skills for UI:", err);
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  return {
    selectSkillsForTask,
    getSkillsContextForPrompt,
    getSkillsSystemPrompt,
    getSkillsForUI,
    loading,
    error,
  };
};
