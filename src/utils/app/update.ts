import type { UpdateSnapshot } from '@/types/update';
import { oeaConfig } from '@/utils/app/config';
import {
  checkForUpdate,
  downloadAndInstallUpdate,
  getUpdateSnapshot,
  onUpdateState,
} from '@/utils/tauri';
import { ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';

export const updateSnapshot = ref<UpdateSnapshot>({
  status: 'idle',
  currentVersion: '0.0.0',
  availableVersion: null,
  releaseNotes: null,
  downloadedBytes: 0,
  totalBytes: null,
  bytesPerSecond: 0,
  error: null,
});
export const updatePopoverOpen = ref(false);
export const installUpdateModalOpen = ref(false);

let initialized = false;

function applySnapshot(snapshot: UpdateSnapshot): void {
  updateSnapshot.value = snapshot;
  if (snapshot.status === 'available' || snapshot.status === 'failed') {
    updatePopoverOpen.value = true;
  }
  if (['downloading', 'verifying', 'preparing', 'bootstrapReady'].includes(snapshot.status)) {
    installUpdateModalOpen.value = true;
  }
}

export async function initUpdateState(): Promise<void> {
  if (initialized) return;
  initialized = true;
  if (!isTauri()) return;
  await onUpdateState(applySnapshot);
  applySnapshot(await getUpdateSnapshot());
  if (oeaConfig.value.checkUpdates) {
    await checkUpdate();
  }
}

export async function checkUpdate(): Promise<void> {
  try {
    await checkForUpdate();
    applySnapshot(await getUpdateSnapshot());
  } catch {
    // Rust emits the authoritative Failed snapshot.
  }
}

export async function downloadAndInstall(): Promise<void> {
  installUpdateModalOpen.value = true;
  updatePopoverOpen.value = false;
  try {
    await downloadAndInstallUpdate();
  } catch {
    // Rust emits preparation failures. Platform launch errors are returned to the caller.
  }
}
