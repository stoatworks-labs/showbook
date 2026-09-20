/**
 * File dialogs: Tauri's inside the app; in the browser, a real file picker for
 * reading and a download for writing. A "path" outside Tauri is a token —
 * `picked://<n>/<name>` for a file the user chose (the File object is held
 * here until the API reads it), `zip://<name>` for a folder of outputs that
 * becomes one zip, or just the file name a download will get.
 */
import { open as tauriOpen, save as tauriSave } from '@tauri-apps/plugin-dialog';

import { inTauri, isLite } from './ipc';
import { hold } from './picked';

function chooseFile(accept: string[]): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept.filter((e) => e !== '*').map((e) => `.${e}`).join(',');
    input.onchange = () => {
      const f = input.files?.[0];
      resolve(f ? hold(f) : null);
    };
    // Safari fires no event when the picker is cancelled; the promise then
    // simply never settles, which is the same as the user not having picked.
    input.click();
  });
}

export async function pickFile(title: string, extensions: string[]): Promise<string | null> {
  if (isLite) return chooseFile(extensions);
  if (!inTauri) return window.prompt(`${title}\n(browser demo: files cannot be read here)`) || null;
  const picked = await tauriOpen({ multiple: false, title, filters: [{ name: title, extensions }, { name: 'All files', extensions: ['*'] }] });
  if (!picked) return null;
  return Array.isArray(picked) ? picked[0] : picked;
}

export async function pickFolder(title: string, defaultName = 'showbook'): Promise<string | null> {
  if (isLite) return `zip://${defaultName}`;
  if (!inTauri) return window.prompt(`${title}\n(browser demo: files are downloaded instead)`, 'downloads') || null;
  const picked = await tauriOpen({ directory: true, title });
  if (!picked) return null;
  return Array.isArray(picked) ? picked[0] : picked;
}

export async function pickSave(title: string, defaultName: string, extensions: string[]): Promise<string | null> {
  if (isLite) return defaultName;
  if (!inTauri) return window.prompt(title, defaultName) || null;
  return tauriSave({ title, defaultPath: defaultName, filters: [{ name: title, extensions }] });
}
