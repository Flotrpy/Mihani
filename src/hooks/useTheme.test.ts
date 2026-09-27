import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useTheme } from "./useTheme";

describe("useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("defaults to light when nothing is stored and system has no preference", () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
  });

  it("reads a previously stored theme instead of the default", () => {
    localStorage.setItem("mihani.theme", "dark");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("dark");
  });

  it("toggleTheme flips between light and dark", () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");

    act(() => result.current.toggleTheme());
    expect(result.current.theme).toBe("dark");

    act(() => result.current.toggleTheme());
    expect(result.current.theme).toBe("light");
  });

  it("persists the theme to localStorage and the document element", () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.toggleTheme());

    expect(localStorage.getItem("mihani.theme")).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("ignores a garbage value in localStorage and falls back to the default", () => {
    localStorage.setItem("mihani.theme", "not-a-real-theme");
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
  });
});
