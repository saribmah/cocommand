interface CacheItem<V> {
  value: V;
  expiresAt: number | null;
}

export class Cache<V = unknown> {
  private store = new Map<string, CacheItem<V>>();
  private namespace: string;

  constructor(namespace?: string) {
    this.namespace = namespace ?? "";
  }

  private key(key: string): string {
    return this.namespace ? `${this.namespace}:${key}` : key;
  }

  private isExpired(item: CacheItem<V>): boolean {
    return item.expiresAt !== null && Date.now() > item.expiresAt;
  }

  async get(key: string): Promise<V | undefined> {
    const item = this.store.get(this.key(key));
    if (!item || this.isExpired(item)) {
      if (item) this.store.delete(this.key(key));
      return undefined;
    }
    return item.value;
  }

  async set(key: string, value: V, options?: { ttl?: number }): Promise<void> {
    const expiresAt = options?.ttl ? Date.now() + options.ttl : null;
    this.store.set(this.key(key), { value, expiresAt });
  }

  async delete(key: string): Promise<boolean> {
    return this.store.delete(this.key(key));
  }

  async has(key: string): Promise<boolean> {
    const item = this.store.get(this.key(key));
    if (!item) return false;
    if (this.isExpired(item)) {
      this.store.delete(this.key(key));
      return false;
    }
    return true;
  }

  async clear(): Promise<void> {
    if (!this.namespace) {
      this.store.clear();
      return;
    }
    const prefix = `${this.namespace}:`;
    for (const k of this.store.keys()) {
      if (k.startsWith(prefix)) this.store.delete(k);
    }
  }

  async keys(): Promise<string[]> {
    const result: string[] = [];
    const prefix = this.namespace ? `${this.namespace}:` : "";
    for (const [k, item] of this.store) {
      if (prefix && !k.startsWith(prefix)) continue;
      if (this.isExpired(item)) {
        this.store.delete(k);
        continue;
      }
      result.push(prefix ? k.slice(prefix.length) : k);
    }
    return result;
  }
}
