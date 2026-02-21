import { describe, expect, it, vi } from "vitest";
import { Computed, createComputed, createSignal, Signal } from "../core/signal";

describe("Signal", () => {
  it("returns initial value", () => {
    const s = createSignal(42);
    expect(s.value).toBe(42);
  });

  it("notifies subscriber on value change", () => {
    const s = createSignal(0);
    const cb = vi.fn();
    s.subscribe(cb);

    s.value = 1;

    expect(cb).toHaveBeenCalledOnce();
    expect(cb).toHaveBeenCalledWith(1);
  });

  it("does not notify when value is unchanged (reference equality)", () => {
    const s = createSignal(5);
    const cb = vi.fn();
    s.subscribe(cb);

    s.value = 5;

    expect(cb).not.toHaveBeenCalled();
  });

  it("stops notifying after unsubscribe", () => {
    const s = createSignal("a");
    const cb = vi.fn();
    const unsub = s.subscribe(cb);

    s.value = "b";
    expect(cb).toHaveBeenCalledOnce();

    unsub();
    s.value = "c";
    expect(cb).toHaveBeenCalledOnce();
  });

  it("supports functional update", () => {
    const s = createSignal(10);
    s.update((v) => v + 5);
    expect(s.value).toBe(15);
  });

  it("notifies multiple subscribers", () => {
    const s = createSignal(0);
    const cb1 = vi.fn();
    const cb2 = vi.fn();
    s.subscribe(cb1);
    s.subscribe(cb2);

    s.value = 1;

    expect(cb1).toHaveBeenCalledWith(1);
    expect(cb2).toHaveBeenCalledWith(1);
  });

  it("is an instance of Signal", () => {
    const s = createSignal(0);
    expect(s).toBeInstanceOf(Signal);
  });
});

describe("Computed", () => {
  it("recomputes when dependency changes", () => {
    const count = createSignal(2);
    const doubled = createComputed(() => count.value * 2, [count]);

    expect(doubled.value).toBe(4);
    count.value = 5;
    expect(doubled.value).toBe(10);
  });

  it("supports unsubscribe", () => {
    const s = createSignal(1);
    const c = createComputed(() => s.value + 1, [s]);
    const cb = vi.fn();
    const unsub = c.subscribe(cb);

    s.value = 2;
    expect(cb).toHaveBeenCalledOnce();

    unsub();
    s.value = 3;
    expect(cb).toHaveBeenCalledOnce();
  });

  it("tracks multiple dependencies", () => {
    const a = createSignal(1);
    const b = createSignal(10);
    const sum = createComputed(() => a.value + b.value, [a, b]);

    expect(sum.value).toBe(11);

    a.value = 2;
    expect(sum.value).toBe(12);

    b.value = 20;
    expect(sum.value).toBe(22);
  });

  it("is an instance of Computed", () => {
    const s = createSignal(0);
    const c = createComputed(() => s.value, [s]);
    expect(c).toBeInstanceOf(Computed);
  });
});
