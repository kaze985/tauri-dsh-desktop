import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface ShellStatus {
  phase: "loading" | "error" | "setup" | "stopped";
  code?: string;
  message?: string;
}

const $ = (id: string) => document.getElementById(id) as HTMLElement;

const views = {
  loading: $("view-loading"),
  error: $("view-error"),
  setup: $("view-setup"),
  stopped: $("view-stopped"),
  close: $("view-close"),
};

function show(view: HTMLElement) {
  for (const v of Object.values(views)) v.classList.toggle("hidden", v !== view);
}

function friendlyCode(code: string): string {
  const map: Record<string, string> = {
    "port-occupied": "端口被占用",
    "spawn-failed": "进程启动失败",
    "startup-timeout": "启动超时",
    "install-failed": "安装失败",
  };
  return map[code] ?? code;
}

function render(status: ShellStatus) {
  switch (status.phase) {
    case "loading":
      ($("loading-text") as HTMLElement).textContent =
        status.message ?? "正在启动 dsh 服务…";
      show(views.loading);
      break;
    case "error": {
      const code = status.code ?? "startup-failed";
      ($("error-message") as HTMLElement).textContent =
        status.message ?? "未知错误，请重试。";
      ($("error-code") as HTMLElement).textContent = friendlyCode(code);
      $("btn-retry").classList.toggle("hidden", code === "port-occupied");
      show(views.error);
      break;
    }
    case "setup":
      show(views.setup);
      break;
    case "stopped":
      ($("stopped-message") as HTMLElement).textContent =
        status.message ?? "dsh 服务意外退出。";
      show(views.stopped);
      break;
  }
}

function renderFromHash() {
  const h = location.hash;
  if (h === "#close-dialog") {
    show(views.close);
  } else if (h === "#stopped") {
    show(views.stopped);
  } else if (h === "#upgrading") {
    ($("loading-text") as HTMLElement).textContent =
      "正在升级 dsh，完成后将重启服务…";
    show(views.loading);
  } else if (h.startsWith("#error")) {
    const q = new URLSearchParams(h.slice(h.indexOf("?") + 1));
    render({
      phase: "error",
      code: q.get("code") ?? undefined,
      message: q.get("message") ?? undefined,
    });
  } else {
    show(views.loading);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  renderFromHash();
  window.addEventListener("hashchange", renderFromHash);

  listen<ShellStatus>("shell://status", (e) => {
    if (location.hash) history.replaceState(null, "", location.pathname);
    render(e.payload);
  });

  $("btn-retry").addEventListener("click", () => invoke("retry_startup"));
  $("btn-exit").addEventListener("click", () => invoke("exit_app"));
  $("btn-restart").addEventListener("click", () => invoke("retry_startup"));
  $("btn-stop-exit").addEventListener("click", () => invoke("exit_app"));
  $("btn-recheck").addEventListener("click", () => invoke("retry_startup"));
  $("btn-setup-exit").addEventListener("click", () => invoke("exit_app"));
  $("btn-install").addEventListener("click", () => invoke("install_dsh"));

  $("btn-close-exit").addEventListener("click", () =>
    invoke("choose_close", {
      action: "exit",
      remember: ($("remember-choice") as HTMLInputElement).checked,
    }),
  );
  $("btn-close-tray").addEventListener("click", () =>
    invoke("choose_close", {
      action: "tray",
      remember: ($("remember-choice") as HTMLInputElement).checked,
    }),
  );
});
