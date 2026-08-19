import { defineStore } from "pinia";
import { reactive, ref } from "vue";

export type ModalName = "create" | "edit" | "del" | "cliCreate" | "cliDel" | "ide" | "provider" | "terminal" | "settings" | "desktopSettings" | "launchLocation";
export type ToastTone = "info" | "error";

export const useUiStore = defineStore("ui", () => {
  const view = ref<"desktop" | "cli">("desktop");
  const toast = reactive({ message: "", tone: "info" as ToastTone, visible: false });
  const modals = reactive<Record<ModalName, boolean>>({
    create: false,
    edit: false,
    del: false,
    cliCreate: false,
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
    return Object.values(modals).some(Boolean);
  }

  function setView(v: "desktop" | "cli") {
    view.value = v;
  }

  function showToast(message: string, tone: ToastTone = "info") {
    toast.message = message;
    toast.tone = tone;
    toast.visible = true;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast.visible = false;
    }, 4200);
  }

  function openModal(name: ModalName) {
    for (const k of Object.keys(modals) as ModalName[]) modals[k] = k === name;
  }

  function closeModal(name: ModalName) {
    modals[name] = false;
  }

  function closeAllModals() {
    for (const k of Object.keys(modals) as ModalName[]) modals[k] = false;
  }

  return { view, toast, modals, anyModalOpen, setView, showToast, openModal, closeModal, closeAllModals };
});
