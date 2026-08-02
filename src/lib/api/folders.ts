import { invoke } from "@tauri-apps/api/core";
import type { Folder, FolderData } from "$lib/types";

function toDto(f: Folder): FolderData {
  return {
    id: f.id,
    name: f.name,
    createdAt: f.createdAt.toISOString(),
    order: f.order,
  };
}

function fromDto(d: FolderData): Folder {
  return {
    id: d.id,
    name: d.name,
    isExpanded: true, // stan UI, nieprzechowywany
    order: d.order,
    createdAt: new Date(d.createdAt),
  };
}

/**
 * Cała lista folderów żyje w jednym zaszyfrowanym pliku. `isExpanded` jest
 * stanem UI i nie trafia na dysk.
 */
export const foldersApi = {
  saveFolders: (folders: Folder[]) =>
    invoke<void>("save_folders", { folders: folders.map(toDto) }),

  loadFolders: async (): Promise<Folder[]> =>
    (await invoke<FolderData[]>("load_folders")).map(fromDto),
};
