import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { hasTauri } from "./lib/tauri";
import "./styles.css";

if (hasTauri && /Mac/i.test(navigator.userAgent)) {
  document.documentElement.setAttribute("data-overlay-controls", "");
}

createApp(App).use(createPinia()).mount("#app");
