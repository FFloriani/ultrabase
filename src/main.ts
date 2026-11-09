import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

const params = new URLSearchParams(location.search);
if (import.meta.env.DEV && params.has("demo")) {
  const { installDemo } = await import("./demo");
  installDemo(params.get("demo") === "vazio");
}

const app = mount(App, { target: document.getElementById("app")! });

export default app;
