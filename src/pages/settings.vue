<script setup lang="ts">
import { getMinimizeToTray, setMinimizeToTray } from '@/lib/tauri';
import { onMounted, ref } from 'vue';

/** 关闭窗口时是否最小化到托盘（与后端状态同步）。 */
const minimizeToTray = ref(false);
/** 正在写入后端设置。 */
const saving = ref(false);

async function loadMinimizeToTray(): Promise<void> {
  minimizeToTray.value = await getMinimizeToTray();
}

/** 切换最小化到托盘：乐观更新，再用后端返回的权威值确认，失败则回滚。 */
async function onMinimizeToTrayChange(value: boolean): Promise<void> {
  saving.value = true;
  try {
    minimizeToTray.value = await setMinimizeToTray(value);
  } catch {
    minimizeToTray.value = !value;
  } finally {
    saving.value = false;
  }
}

onMounted(loadMinimizeToTray);
</script>

<template>
  <UContainer>
    <UPage>
      <UPageHeader description="OEA 应用设置" title="设置" />

      <UPageBody>
        <UCard description="托盘图标与窗口行为" title="系统托盘">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
              <span class="i-lucide-panel-bottom-dashed text-2xl text-primary" />
              <div>
                <p class="font-medium">关闭时最小化到托盘</p>
                <p class="text-sm text-toned">
                  点击窗口关闭按钮时隐藏到系统托盘而不是退出，可通过托盘菜单或 Alt+Delete 退出
                </p>
              </div>
            </div>
            <USwitch
              v-model="minimizeToTray"
              :loading="saving"
              @update:model-value="onMinimizeToTrayChange"
            />
          </div>
        </UCard>
      </UPageBody>
    </UPage>
  </UContainer>
</template>
