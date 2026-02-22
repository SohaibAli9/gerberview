import type { AppStore } from "../core/store";

function dismissError(store: AppStore): void {
  store.error.value = null;
}

/**
 * Initialize the error banner that displays store errors as a
 * dismissable red overlay. Supports close button and Escape key.
 */
export function setupErrorBanner(container: HTMLElement, store: AppStore): void {
  const inner = document.createElement("div");
  inner.className =
    "flex items-center justify-between gap-4 px-4 py-3 bg-red-900/90 text-red-100 text-sm";

  const message = document.createElement("p");
  message.className = "flex-1";

  const closeBtn = document.createElement("button");
  closeBtn.type = "button";
  closeBtn.className =
    "shrink-0 text-red-300 hover:text-white text-lg leading-none focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400 rounded px-1";
  closeBtn.textContent = "\u00d7";
  closeBtn.setAttribute("aria-label", "Dismiss error");

  closeBtn.addEventListener("click", () => {
    dismissError(store);
  });

  inner.append(message, closeBtn);
  container.appendChild(inner);

  store.error.subscribe((error) => {
    if (error !== null) {
      message.textContent = error.message;
      container.classList.remove("hidden");
    } else {
      container.classList.add("hidden");
    }
  });

  document.addEventListener("keydown", (e: KeyboardEvent) => {
    if (e.key === "Escape" && store.error.value !== null) {
      dismissError(store);
    }
  });
}
