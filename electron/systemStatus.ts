import si from 'systeminformation';
import type { WebContents } from 'electron';

export interface SystemStats {
  cpuUsage: number;
  cpuTemp: number | null;
  ramUsage: {
    used: number;
    total: number;
    percent: number;
  };
  gpu: {
    usage: number | null;
    temp: number | null;
    name: string | null;
    memoryUsed: number | null;
    memoryTotal: number | null;
  };
}

export class SystemStatusService {
  private interval: NodeJS.Timeout | null = null;
  private webContents: WebContents | null = null;

  constructor(webContents: WebContents) {
    this.webContents = webContents;
  }

  start(intervalMs: number = 3000) {
    this.stop();
    this.interval = setInterval(async () => {
      try {
        const stats = await this.getStats();
        if (this.webContents && !this.webContents.isDestroyed()) {
          this.webContents.send('system:status', stats);
        }
      } catch (error) {
        // Silently handle errors to prevent flooding logs
      }
    }, intervalMs);
  }

  stop() {
    if (this.interval) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }

  async getStats(): Promise<SystemStats> {
    // Fetch multiple stats in parallel
    const [cpu, temp, mem, graphics] = await Promise.all([
      si.currentLoad(),
      si.cpuTemperature(),
      si.mem(),
      si.graphics()
    ]);

    // Find the primary GPU (usually the first one with utilization or just the first one)
    const gpuController = graphics.controllers && graphics.controllers.length > 0 
      ? graphics.controllers.find(c => c.utilizationGpu !== undefined) || graphics.controllers[0]
      : null;

    return {
      cpuUsage: Math.round(cpu.currentLoad),
      cpuTemp: temp.main !== -1 && temp.main !== null ? Math.round(temp.main) : null,
      ramUsage: {
        used: mem.used,
        total: mem.total,
        percent: Math.round((mem.used / mem.total) * 100)
      },
      gpu: {
        usage: gpuController?.utilizationGpu !== undefined ? Math.round(gpuController.utilizationGpu) : null,
        temp: gpuController?.temperatureGpu !== undefined && gpuController.temperatureGpu !== -1 ? Math.round(gpuController.temperatureGpu) : null,
        name: gpuController?.model || null,
        memoryUsed: gpuController?.memoryUsed !== undefined ? gpuController.memoryUsed : null,
        memoryTotal: gpuController?.memoryTotal !== undefined ? gpuController.memoryTotal : null
      }
    };
  }
}
