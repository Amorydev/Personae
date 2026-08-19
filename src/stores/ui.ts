import { defineStore } from "pinia";
import { reactive, ref } from "vue";

export type ModalName = "create" | "edit" | "del" | "cliCreate" | "cliEdit" | "cliDel" | "ide" | "provider" | "terminal" | "settings" | "desktopSettings" | "launchLocation";
export type ToastTone = "info" | "error";

export const useUiStore = defineStore("ui", () => {
  const view = ref<"desktop" | "cli">("desktop");
  const toast = reactive({ message: "", tone: "info" as ToastTone, visible: false });
  // Errors get a dismissible modal, not an auto-hiding toast — a toast can be
  // missed or time out before someone's read the actual message, and error
  // text (e.g. a raw backend error string) is often exactly what someone
  // needs to read carefully, not a passing notification.
  const errorMessage = ref<string | null>(null);
  const modals = reactive<Record<ModalName, boolean>>({
    create: false,
    edit: false,
    del: false,
    cliCreate: false,
    cliEdit: false,
    cliDel: false,
    ide: false,
    provider: false,
    terminal: false,
    settings: false,
    desktopSettings: false,
    launchLocation: false,
  });
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function anyModalOpen(): boolean {
    return errorMessage.value !== null || Object.values(modals).some(Boolean);
  }

  function setView(v: "desktop" | "cli") {
    view.value = v;
  }

  // tone: "error" opens the error modal instead of a toast — see
  // errorMessage's doc comment. Existing call sites (ui.showToast(msg,
  // "error")) don't need to change; this is the single place that decides
  // how an error actually gets shown.
  function showToast(message: string, tone: ToastTone = "info") {
    if (tone === "error") {
      errorMessage.value = message;
      return;
    }
    toast.message = message;
    toast.tone = tone;
    toast.visible = true;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast.visible = false;
    }, 4200);
  }

  function closeError() {
    errorMessage.value = null;
  }

  function openModal(name: ModalName) {
    modals[name] = true;
  }

  function closeModal(name: ModalName) {
    modals[name] = false;
  }

  function closeAllModals() {
    errorMessage.value = null;
    for (const k of Object.keys(modals) as ModalName[]) modals[k] = false;
  }

  return { view, toast, modals, errorMessage, anyModalOpen, setView, showToast, closeError, openModal, closeModal, closeAllModals };
});
