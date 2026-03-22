import React from 'react';

export interface StreamingPhase {
  name: string;
  startTime: number;
  endTime?: number;
  status: 'pending' | 'active' | 'completed' | 'failed';
}

export interface StreamingStatusState {
  isActive: boolean;
  currentPhase: string;
  phases: StreamingPhase[];
  elapsedSeconds: number;
  progress: number;
}

export const useStreamingStatus = () => {
  const [status, setStatus] = React.useState<StreamingStatusState>({
    isActive: false,
    currentPhase: 'initializing',
    phases: [],
    elapsedSeconds: 0,
    progress: 0,
  });

  const startStreaming = (initialPhase: string = 'analyzing') => {
    setStatus({
      isActive: true,
      currentPhase: initialPhase,
      phases: [{ name: initialPhase, startTime: Date.now(), status: 'active' }],
      elapsedSeconds: 0,
      progress: 0,
    });
  };

  const updatePhase = (newPhase: string) => {
    setStatus((prev) => {
      const updatedPhases = [...prev.phases];
      if (updatedPhases.length > 0) {
        updatedPhases[updatedPhases.length - 1].endTime = Date.now();
        updatedPhases[updatedPhases.length - 1].status = 'completed';
      }

      return {
        ...prev,
        currentPhase: newPhase,
        phases: [
          ...updatedPhases,
          { name: newPhase, startTime: Date.now(), status: 'active' },
        ],
        progress: Math.min(prev.progress + 0.2, 0.9),
      };
    });
  };

  const completePhase = (phaseName?: string) => {
    setStatus((prev) => {
      const updatedPhases = [...prev.phases];
      const targetIdx = phaseName
        ? updatedPhases.findIndex((p) => p.name === phaseName)
        : updatedPhases.length - 1;

      if (targetIdx >= 0) {
        updatedPhases[targetIdx].endTime = Date.now();
        updatedPhases[targetIdx].status = 'completed';
      }

      return {
        ...prev,
        phases: updatedPhases,
        progress: Math.min(prev.progress + 0.1, 0.95),
      };
    });
  };

  const failPhase = (phaseName?: string, error?: string) => {
    setStatus((prev) => {
      const updatedPhases = [...prev.phases];
      const targetIdx = phaseName
        ? updatedPhases.findIndex((p) => p.name === phaseName)
        : updatedPhases.length - 1;

      if (targetIdx >= 0) {
        updatedPhases[targetIdx].endTime = Date.now();
        updatedPhases[targetIdx].status = 'failed';
      }

      return {
        ...prev,
        phases: updatedPhases,
      };
    });
  };

  const stopStreaming = () => {
    setStatus((prev) => ({
      ...prev,
      isActive: false,
      progress: 1,
      phases: prev.phases.map((p) => ({
        ...p,
        endTime: p.endTime || Date.now(),
        status: p.status === 'active' ? 'completed' : p.status,
      })),
    }));
  };

  const resetStreaming = () => {
    setStatus({
      isActive: false,
      currentPhase: 'initializing',
      phases: [],
      elapsedSeconds: 0,
      progress: 0,
    });
  };

  // Timer effect
  React.useEffect(() => {
    if (!status.isActive) return;

    const interval = setInterval(() => {
      setStatus((prev) => ({
        ...prev,
        elapsedSeconds: prev.elapsedSeconds + 1,
      }));
    }, 1000);

    return () => clearInterval(interval);
  }, [status.isActive]);

  const getPhaseNames = (): string[] => {
    return status.phases.map((p) => p.name);
  };

  const getPhaseStats = () => {
    return status.phases.map((phase) => ({
      name: phase.name,
      duration: (phase.endTime || Date.now()) - phase.startTime,
      status: phase.status,
    }));
  };

  return {
    status,
    startStreaming,
    updatePhase,
    completePhase,
    failPhase,
    stopStreaming,
    resetStreaming,
    getPhaseNames,
    getPhaseStats,
  };
};
