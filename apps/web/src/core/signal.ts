export interface ReadonlySignal<T> {
  readonly value: T;
  subscribe(fn: (value: T) => void): () => void;
}

type Subscriber<T> = (value: T) => void;

export class Signal<T> implements ReadonlySignal<T> {
  private _value: T;
  private readonly _subscribers = new Set<Subscriber<T>>();

  constructor(initial: T) {
    this._value = initial;
  }

  get value(): T {
    return this._value;
  }

  set value(next: T) {
    if (next !== this._value) {
      this._value = next;
      this._notify();
    }
  }

  update(fn: (current: T) => T): void {
    this.value = fn(this._value);
  }

  subscribe(fn: Subscriber<T>): () => void {
    this._subscribers.add(fn);
    return () => {
      this._subscribers.delete(fn);
    };
  }

  private _notify(): void {
    for (const fn of this._subscribers) {
      fn(this._value);
    }
  }
}

export class Computed<T> implements ReadonlySignal<T> {
  private _value: T;
  private readonly _subscribers = new Set<Subscriber<T>>();
  private readonly _unsubscribers: (() => void)[] = [];

  constructor(
    private readonly _compute: () => T,
    deps: readonly ReadonlySignal<unknown>[],
  ) {
    this._value = _compute();

    for (const dep of deps) {
      this._unsubscribers.push(
        dep.subscribe(() => {
          this._recompute();
        }),
      );
    }
  }

  get value(): T {
    return this._value;
  }

  subscribe(fn: Subscriber<T>): () => void {
    this._subscribers.add(fn);
    return () => {
      this._subscribers.delete(fn);
    };
  }

  destroy(): void {
    for (const unsub of this._unsubscribers) {
      unsub();
    }
    this._unsubscribers.length = 0;
    this._subscribers.clear();
  }

  private _recompute(): void {
    const next = this._compute();
    if (next !== this._value) {
      this._value = next;
      for (const fn of this._subscribers) {
        fn(this._value);
      }
    }
  }
}

export function createSignal<T>(initial: T): Signal<T> {
  return new Signal(initial);
}

export function createComputed<T>(
  fn: () => T,
  deps: readonly ReadonlySignal<unknown>[],
): Computed<T> {
  return new Computed(fn, deps);
}
