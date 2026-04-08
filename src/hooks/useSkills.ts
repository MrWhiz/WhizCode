import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Skill {
  name: string;
  description: string;
  version: string;
  author: string;
  enabled: boolean;
}

interface UseSkillsReturn {
  skills: Skill[];
  loading: boolean;
  error: string | null;
  loadSkills: () => Promise<void>;
  refreshSkills: () => Promise<void>;
}

/**
 * Custom hook for managing skills
 *
 * Handles loading and refreshing skills from the global cache.
 * Skills are cached globally in ~/.whizcode/skills/cache/
 *
 * @returns {UseSkillsReturn} Skills state and operations
 */
export const useSkills = (): UseSkillsReturn => {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /**
   * Load skills from global cache
   */
  const loadSkills = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const loadedSkills = await invoke<Skill[]>("get_skills");
      setSkills(loadedSkills);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to load skills: ${errorMessage}`);
      console.error("Error loading skills:", err);
      setSkills([]);
    } finally {
      setLoading(false);
    }
  }, []);

  /**
   * Refresh skills from repository
   */
  const refreshSkills = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const refreshedSkills = await invoke<Skill[]>("refresh_skills");
      setSkills(refreshedSkills);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to refresh skills: ${errorMessage}`);
      console.error("Error refreshing skills:", err);
      setSkills([]);
    } finally {
      setLoading(false);
    }
  }, []);

  /**
   * Load skills on component mount
   */
  useEffect(() => {
    loadSkills();
  }, [loadSkills]);

  return {
    skills,
    loading,
    error,
    loadSkills,
    refreshSkills,
  };
};
