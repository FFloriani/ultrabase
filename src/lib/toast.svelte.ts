export interface Toast {
  id: number;
  text: string;
  kind: "ok" | "error";
}

export const toasts: Toast[] = $state([]);

let next = 0;

export function notify(text: string, kind: Toast["kind"] = "ok") {
  const id = ++next;
  toasts.push({ id, text, kind });
  setTimeout(() => dismiss(id), kind === "error" ? 8000 : 3000);
}

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

export async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    notify(`${what} copiado`);
  } catch {
    notify(`Não deu para copiar ${what.toLowerCase()}`, "error");
  }
}
