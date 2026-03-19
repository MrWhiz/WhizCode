/**
 * Timeout utilities to prevent agents from getting stuck
 * Provides timeout wrappers for async operations
 */

export interface TimeoutOptions {
  timeoutMs: number;
  onTimeout?: () => void;
  fallbackValue?: any;
}

/**
 * Wraps a promise with a timeout
 * @param promise The promise to wrap
 * @param timeoutMs Timeout in milliseconds
 * @param timeoutMessage Error message to throw on timeout
 * @returns Promise that rejects if timeout is exceeded
 */
export function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  timeoutMessage: string = 'Operation timed out'
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs)
    )
  ]);
}

/**
 * Wraps a promise with a timeout and fallback value
 * @param promise The promise to wrap
 * @param timeoutMs Timeout in milliseconds
 * @param fallbackValue Value to return on timeout
 * @returns Promise that returns fallback value if timeout is exceeded
 */
export function withTimeoutFallback<T>(
  promise: Promise<T>,
  timeoutMs: number,
  fallbackValue: T
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>(resolve =>
      setTimeout(() => resolve(fallbackValue), timeoutMs)
    )
  ]);
}

/**
 * Creates a cancellable promise wrapper
 */
export class CancellablePromise<T> {
  private cancelled = false;
  private promise: Promise<T>;
  private reject?: (reason?: any) => void;

  constructor(executor: (resolve: (value: T) => void, reject: (reason?: any) => void) => void) {
    this.promise = new Promise((resolve, reject) => {
      this.reject = reject;
      executor(
        (value: T) => {
          if (!this.cancelled) resolve(value);
        },
        (reason?: any) => {
          if (!this.cancelled) reject(reason);
        }
      );
    });
  }

  cancel(): void {
    this.cancelled = true;
    this.reject?.(new Error('Promise cancelled'));
  }

  getPromise(): Promise<T> {
    return this.promise;
  }

  isCancelled(): boolean {
    return this.cancelled;
  }
}

/**
 * Manages a set of active promises with timeout protection
 */
export class PromiseManager {
  private activePromises: Map<string, { promise: Promise<any>; timeout: NodeJS.Timeout }> = new Map();
  private defaultTimeoutMs = 30000; // 30 seconds default

  /**
   * Track a promise with automatic timeout
   */
  track<T>(
    id: string,
    promise: Promise<T>,
    timeoutMs: number = this.defaultTimeoutMs,
    onTimeout?: () => void
  ): Promise<T> {
    // Clear any existing promise with this ID
    this.clear(id);

    const timeout = setTimeout(() => {
      console.warn(`[PROMISE_MANAGER] Promise ${id} timed out after ${timeoutMs}ms`);
      onTimeout?.();
      this.activePromises.delete(id);
    }, timeoutMs);

    const wrappedPromise = promise
      .then(result => {
        this.activePromises.delete(id);
        clearTimeout(timeout);
        return result;
      })
      .catch(error => {
        this.activePromises.delete(id);
        clearTimeout(timeout);
        throw error;
      });

    this.activePromises.set(id, { promise: wrappedPromise, timeout });
    return wrappedPromise;
  }

  /**
   * Clear a tracked promise
   */
  clear(id: string): void {
    const entry = this.activePromises.get(id);
    if (entry) {
      clearTimeout(entry.timeout);
      this.activePromises.delete(id);
    }
  }

  /**
   * Clear all tracked promises
   */
  clearAll(): void {
    for (const entry of this.activePromises.values()) {
      clearTimeout(entry.timeout);
    }
    this.activePromises.clear();
  }

  /**
   * Get count of active promises
   */
  getActiveCount(): number {
    return this.activePromises.size;
  }

  /**
   * Get list of active promise IDs
   */
  getActiveIds(): string[] {
    return Array.from(this.activePromises.keys());
  }
}

/**
 * Circular buffer for terminal output to prevent unbounded growth
 */
export class CircularBuffer<T> {
  private buffer: T[] = [];
  private maxSize: number;
  private writeIndex = 0;

  constructor(maxSize: number = 10000) {
    this.maxSize = maxSize;
  }

  push(item: T): void {
    if (this.buffer.length < this.maxSize) {
      this.buffer.push(item);
    } else {
      this.buffer[this.writeIndex] = item;
      this.writeIndex = (this.writeIndex + 1) % this.maxSize;
    }
  }

  getAll(): T[] {
    if (this.buffer.length < this.maxSize) {
      return [...this.buffer];
    }
    // Return in correct order when buffer is full
    return [
      ...this.buffer.slice(this.writeIndex),
      ...this.buffer.slice(0, this.writeIndex)
    ];
  }

  getLast(count: number): T[] {
    const all = this.getAll();
    return all.slice(Math.max(0, all.length - count));
  }

  clear(): void {
    this.buffer = [];
    this.writeIndex = 0;
  }

  size(): number {
    return this.buffer.length;
  }

  isFull(): boolean {
    return this.buffer.length >= this.maxSize;
  }
}

/**
 * Debounced function executor
 */
export class DebouncedExecutor {
  private timeouts: Map<string, NodeJS.Timeout> = new Map();

  execute<T>(
    id: string,
    fn: () => Promise<T>,
    delayMs: number = 500
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      // Clear existing timeout for this ID
      const existing = this.timeouts.get(id);
      if (existing) clearTimeout(existing);

      const timeout = setTimeout(async () => {
        this.timeouts.delete(id);
        try {
          const result = await fn();
          resolve(result);
        } catch (error) {
          reject(error);
        }
      }, delayMs);

      this.timeouts.set(id, timeout);
    });
  }

  cancel(id: string): void {
    const timeout = this.timeouts.get(id);
    if (timeout) {
      clearTimeout(timeout);
      this.timeouts.delete(id);
    }
  }

  cancelAll(): void {
    for (const timeout of this.timeouts.values()) {
      clearTimeout(timeout);
    }
    this.timeouts.clear();
  }
}
