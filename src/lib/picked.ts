/**
 * Files the user chose in the browser, held until the API reads them. A
 * `picked://<n>/<name>` token stands in for the path the desktop app would
 * have. Its own module so dialogs.ts and lite.ts can both use it without
 * importing each other (ipc.ts awaits lite.ts at load; a cycle through it
 * would hang the page before it drew anything).
 */
const picked = new Map<string, File>();
let counter = 0;

export function hold(file: File): string {
  const token = `picked://${++counter}/${file.name}`;
  picked.set(token, file);
  return token;
}

/** The File behind a token, once. */
export function take(token: string): File | undefined {
  const f = picked.get(token);
  picked.delete(token);
  return f;
}
