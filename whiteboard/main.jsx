import React from "react";
import { createRoot } from "react-dom/client";
import { Excalidraw } from "excalidraw-local";
import "excalidraw-local/index.css";
import "./whiteboard.css";

const init = window.__WB_INIT__ || {};
const tauri = window.__TAURI__;
let saveTimer = null;

function scheduleSave(elements, appState, files) {
  if (!tauri || !init.id) return;
  if (saveTimer) clearTimeout(saveTimer);

  saveTimer = setTimeout(() => {
    const cleanAppState = { ...appState };
    delete cleanAppState.collaborators;
    const sceneJson = JSON.stringify({
      type: "excalidraw",
      version: 2,
      source: "rune",
      elements,
      appState: cleanAppState,
      files: files || {},
    });

    tauri.core
      .invoke("save_whiteboard_from_window", {
        whiteboardId: init.id,
        noteId: init.noteId,
        sceneJson,
      })
      .catch(() => {});
  }, 1000);
}

function parseScene(json) {
  try {
    const scene = JSON.parse(json || "{}");
    if (scene?.appState) delete scene.appState.collaborators;
    if (scene && (scene.elements || scene.appState)) {
      return {
        elements: scene.elements || [],
        appState: scene.appState || {},
        files: scene.files || {},
      };
    }
  } catch {
    // A damaged or empty scene opens as a clean board.
  }
  return null;
}

function render(initialData) {
  createRoot(document.getElementById("root")).render(
    <Excalidraw
      theme="dark"
      initialData={initialData}
      onChange={scheduleSave}
    />,
  );
}

if (tauri && init.id) {
  tauri.core
    .invoke("load_whiteboard", { whiteboardId: init.id })
    .then((data) => render(parseScene(data?.excalidrawJson)))
    .catch(() => render(null));
} else {
  render(null);
}
