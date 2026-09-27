import "@testing-library/jest-dom/vitest";

// jsdom doesn't implement matchMedia; several hooks/components check
// prefers-color-scheme, so stub it to "no preference" by default.
if (!window.matchMedia) {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as MediaQueryList;
}

