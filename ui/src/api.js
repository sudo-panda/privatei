import { invoke } from '@tauri-apps/api/core';
import { toast } from './dom.js';

export function message(e) {
  if (typeof e === 'string') return e;
  if (e && typeof e.message === 'string') return e.message;
  return 'Something went wrong.';
}

/** Call a command; on failure show a short message and rethrow. */
export async function call(cmd, args) {
  try {
    return await invoke(cmd, args);
  } catch (e) {
    toast(message(e), 'error');
    throw e;
  }
}

/** Call a command and let the caller deal with errors. */
export const quiet = (cmd, args) => invoke(cmd, args);
