import { mountReactApp } from "./react-app-shell.tsx";

export * from "./shell-controller.ts";
export * from "./react-app-shell.tsx";
export * from "./react-action-adapters.tsx";

if (typeof window !== "undefined") {
  window.addEventListener("DOMContentLoaded", () => {
    const root = document.querySelector<HTMLElement>("#app");
    if (root) {
      void mountReactApp(root);
    }
  });
}
